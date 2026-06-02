use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Deserialize)]
pub struct CreateQuestionRequest {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_metadata")]
    pub metadata: serde_json::Value,
}

fn default_metadata() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Serialize)]
pub struct QuestionResponse {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub body: String,
    pub original_query: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub status: String,
    pub has_embedding: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_question(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateQuestionRequest>,
) -> Result<(StatusCode, Json<QuestionResponse>), StatusCode> {
    let original_query = format!("{} {}", req.title, req.body);

    let row = sqlx::query_as::<_, (Uuid, String, String, String, Vec<String>, serde_json::Value, String, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO questions (author_id, title, body, original_query, tags, metadata)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, title, body, original_query, tags, metadata, status, created_at"#,
    )
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&req.body)
    .bind(&original_query)
    .bind(&req.tags)
    .bind(&req.metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("failed to insert question: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Optionally generate embedding in background
    if let Some(model) = &state.llm_embedding_model {
        let db = state.db.clone();
        let model = model.clone();
        let text = original_query.clone();
        let question_id = row.0;

        tokio::spawn(async move {
            if let Err(e) = generate_embedding(&db, &model, question_id, &text).await {
                tracing::error!("embedding generation failed for {}: {}", question_id, e);
            }
        });
    }

    Ok((
        StatusCode::CREATED,
        Json(QuestionResponse {
            id: row.0,
            author_id: auth.user_id,
            title: row.1,
            body: row.2,
            original_query: row.3,
            tags: row.4,
            metadata: row.5,
            status: row.6,
            has_embedding: false, // async, not ready yet
            created_at: row.7,
        }),
    ))
}

pub async fn get_question(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<QuestionResponse>, StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Vec<String>, serde_json::Value, String, bool, chrono::DateTime<chrono::Utc>)>(
        r#"SELECT id, author_id, title, body, original_query, tags, metadata, status, embedding IS NOT NULL, created_at
           FROM questions WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(QuestionResponse {
        id: row.0,
        author_id: row.1,
        title: row.2,
        body: row.3,
        original_query: row.4,
        tags: row.5,
        metadata: row.6,
        status: row.7,
        has_embedding: row.8,
        created_at: row.9,
    }))
}

async fn generate_embedding(
    db: &sqlx::PgPool,
    model: &str,
    question_id: Uuid,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use pgvector::Vector;

    let client = genai::Client::default();
    let resp = client.embed(model, text, None).await?;

    let vector = Vector::from(resp.embeddings[0].vector.clone());

    sqlx::query("UPDATE questions SET embedding = $1 WHERE id = $2")
        .bind(vector)
        .bind(question_id)
        .execute(db)
        .await?;

    tracing::info!("embedding generated for question {}", question_id);
    Ok(())
}
