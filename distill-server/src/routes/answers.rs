use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Serialize)]
pub struct AnswerResponse {
    pub id: Uuid,
    pub question_id: Uuid,
    pub author_id: Option<Uuid>,
    pub author_type: String,
    pub body: String,
    pub is_stale: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct EditAnswerRequest {
    pub body: String,
    pub edit_message: Option<String>,
}

#[derive(Serialize)]
pub struct EditHistoryEntry {
    pub id: Uuid,
    pub editor_id: Uuid,
    pub diff: String,
    pub edit_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn edit_answer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<EditAnswerRequest>,
) -> Result<Json<AnswerResponse>, StatusCode> {
    // Get current body
    let old_body: (String,) = sqlx::query_as("SELECT body FROM answers WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| { tracing::error!("edit fetch failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Compute diff
    let patch = diffy_imara::create_patch(&old_body.0, &req.body);
    let diff_text = patch.to_string();

    // Store diff
    sqlx::query(
        "INSERT INTO answer_edits (answer_id, editor_id, diff, edit_message) VALUES ($1, $2, $3, $4)"
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&diff_text)
    .bind(&req.edit_message)
    .execute(&state.db)
    .await
    .map_err(|e| { tracing::error!("edit insert failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    // Update answer body
    let row = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, String, bool, chrono::DateTime<chrono::Utc>)>(
        r#"UPDATE answers SET body = $1, updated_at = now() WHERE id = $2
           RETURNING id, question_id, author_id, author_type, body, is_stale, created_at"#,
    )
    .bind(&req.body)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| { tracing::error!("edit update failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(AnswerResponse {
        id: row.0, question_id: row.1, author_id: row.2, author_type: row.3,
        body: row.4, is_stale: row.5, created_at: row.6,
    }))
}

pub async fn get_history(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<EditHistoryEntry>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, editor_id, diff, edit_message, created_at FROM answer_edits WHERE answer_id = $1 ORDER BY created_at ASC"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("history fetch failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(rows.into_iter().map(|r| EditHistoryEntry {
        id: r.0, editor_id: r.1, diff: r.2, edit_message: r.3, created_at: r.4,
    }).collect()))
}

pub async fn get_answers(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<Vec<AnswerResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, String, bool, chrono::DateTime<chrono::Utc>)>(
        r#"SELECT id, question_id, author_id, author_type, body, is_stale, created_at
           FROM answers WHERE question_id = $1 ORDER BY created_at ASC"#,
    )
    .bind(question_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("failed to fetch answers: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(
        rows.into_iter()
            .map(|r| AnswerResponse {
                id: r.0,
                question_id: r.1,
                author_id: r.2,
                author_type: r.3,
                body: r.4,
                is_stale: r.5,
                created_at: r.6,
            })
            .collect(),
    ))
}

pub async fn generate_ai_answer(
    db: &sqlx::PgPool,
    chat_model: &str,
    question_id: Uuid,
    title: &str,
    body: &str,
) {
    if let Err(e) = do_generate_ai_answer(db, chat_model, question_id, title, body).await {
        tracing::error!("AI answer generation failed for {}: {:?}", question_id, e);
    }
}

async fn do_generate_ai_answer(
    db: &sqlx::PgPool,
    chat_model: &str,
    question_id: Uuid,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use genai::chat::{ChatMessage, ChatRequest};

    // Gather context: find similar Q&A pairs
    let context_rows = sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT q.title, q.body, a.body
           FROM answers a JOIN questions q ON q.id = a.question_id
           WHERE q.id != $1
           ORDER BY q.created_at DESC LIMIT 5"#,
    )
    .bind(question_id)
    .fetch_all(db)
    .await?;

    let mut context = String::new();
    for (qt, qb, ab) in &context_rows {
        context.push_str(&format!("Q: {} - {}\nA: {}\n\n", qt, qb, ab));
    }

    let system_prompt = if context.is_empty() {
        "You are a knowledgeable assistant. Answer the question clearly and concisely.".to_string()
    } else {
        format!(
            "You are a knowledgeable assistant. Here is some relevant context from existing Q&A:\n\n{}\nAnswer the following question clearly and concisely.",
            context
        )
    };

    let client = genai::Client::default();
    let chat_req = ChatRequest::new(vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(format!("Question: {}\n\n{}", title, body)),
    ]);

    let resp = client.exec_chat(chat_model, chat_req, None).await?;
    let answer_text = resp.first_text()
        .ok_or("no text in LLM response")?
        .to_string();

    sqlx::query(
        r#"INSERT INTO answers (question_id, author_type, body)
           VALUES ($1, 'ai', $2)"#,
    )
    .bind(question_id)
    .bind(&answer_text)
    .execute(db)
    .await?;

    tracing::info!("AI answer generated for question {}", question_id);
    Ok(())
}
