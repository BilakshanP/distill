use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;

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
