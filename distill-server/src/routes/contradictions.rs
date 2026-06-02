use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Serialize)]
pub struct ContradictionResponse {
    pub id: Uuid,
    pub answer_id_a: Uuid,
    pub answer_id_b: Uuid,
    pub explanation: String,
    pub source: String,
    pub flagged_by: Option<Uuid>,
    pub status: String,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct FlagContradictionRequest {
    pub contradicts_answer_id: Uuid,
    pub explanation: String,
}

pub async fn flag_contradiction(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<FlagContradictionRequest>,
) -> Result<(StatusCode, Json<ContradictionResponse>), StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, String, Option<Uuid>, String, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO contradiction_flags (answer_id_a, answer_id_b, explanation, source, flagged_by)
           VALUES ($1, $2, $3, 'user', $4)
           RETURNING id, answer_id_a, answer_id_b, explanation, source, flagged_by, status, detected_at"#,
    )
    .bind(answer_id)
    .bind(req.contradicts_answer_id)
    .bind(&req.explanation)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| { tracing::error!("flag contradiction failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok((StatusCode::CREATED, Json(ContradictionResponse {
        id: row.0, answer_id_a: row.1, answer_id_b: row.2, explanation: row.3,
        source: row.4, flagged_by: row.5, status: row.6, detected_at: row.7,
    })))
}

pub async fn get_contradictions_for_answer(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
) -> Result<Json<Vec<ContradictionResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, String, Option<Uuid>, String, chrono::DateTime<chrono::Utc>)>(
        r#"SELECT id, answer_id_a, answer_id_b, explanation, source, flagged_by, status, detected_at
           FROM contradiction_flags WHERE answer_id_a = $1 OR answer_id_b = $1
           ORDER BY detected_at DESC"#,
    )
    .bind(answer_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("get contradictions failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(rows.into_iter().map(|r| ContradictionResponse {
        id: r.0, answer_id_a: r.1, answer_id_b: r.2, explanation: r.3,
        source: r.4, flagged_by: r.5, status: r.6, detected_at: r.7,
    }).collect()))
}

pub async fn admin_review_queue(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Vec<ContradictionResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, String, Option<Uuid>, String, chrono::DateTime<chrono::Utc>)>(
        r#"SELECT id, answer_id_a, answer_id_b, explanation, source, flagged_by, status, detected_at
           FROM contradiction_flags WHERE status = 'pending'
           ORDER BY detected_at ASC"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("admin queue failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(rows.into_iter().map(|r| ContradictionResponse {
        id: r.0, answer_id_a: r.1, answer_id_b: r.2, explanation: r.3,
        source: r.4, flagged_by: r.5, status: r.6, detected_at: r.7,
    }).collect()))
}

/// Auto-detect contradictions for a newly created answer
pub async fn detect_contradictions(
    db: &sqlx::PgPool,
    chat_model: &str,
    answer_id: Uuid,
    answer_body: &str,
    question_id: Uuid,
) {
    if let Err(e) = do_detect(db, chat_model, answer_id, answer_body, question_id).await {
        tracing::error!("contradiction detection failed for {}: {:?}", answer_id, e);
    }
}

async fn do_detect(
    db: &sqlx::PgPool,
    chat_model: &str,
    answer_id: Uuid,
    answer_body: &str,
    question_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use genai::chat::{ChatMessage, ChatRequest};

    // Get other answers to the same or similar questions
    let other_answers = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, body FROM answers WHERE question_id = $1 AND id != $2",
    )
    .bind(question_id)
    .bind(answer_id)
    .fetch_all(db)
    .await?;

    if other_answers.is_empty() {
        return Ok(());
    }

    let client = genai::Client::default();

    for (other_id, other_body) in &other_answers {
        let chat_req = ChatRequest::new(vec![
            ChatMessage::system("You are a contradiction detector. Compare two answers and determine if they contradict each other. Reply ONLY with 'NO' if they don't contradict, or a brief explanation of the contradiction if they do."),
            ChatMessage::user(format!("Answer A:\n{}\n\nAnswer B:\n{}", answer_body, other_body)),
        ]);

        let resp = client.exec_chat(chat_model, chat_req, None).await?;
        if let Some(text) = resp.first_text() {
            let text = text.trim();
            if text != "NO" && !text.to_lowercase().starts_with("no") {
                // Contradiction found
                sqlx::query(
                    r#"INSERT INTO contradiction_flags (answer_id_a, answer_id_b, explanation, source)
                       VALUES ($1, $2, $3, 'auto')"#,
                )
                .bind(answer_id)
                .bind(other_id)
                .bind(text)
                .execute(db)
                .await?;

                tracing::info!("contradiction detected between {} and {}", answer_id, other_id);
            }
        }
    }

    Ok(())
}
