use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Deserialize)]
pub struct CreateCommentRequest {
    pub body: String,
}

#[derive(Serialize)]
pub struct CommentResponse {
    pub id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_question_comment(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<CommentResponse>), StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO comments (author_id, body, question_id)
           VALUES ($1, $2, $3)
           RETURNING id, author_id, body, created_at"#,
    )
    .bind(auth.user_id)
    .bind(&req.body)
    .bind(question_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("create question comment failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CommentResponse {
            id: row.0,
            author_id: row.1,
            body: row.2,
            created_at: row.3,
        }),
    ))
}

pub async fn get_question_comments(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<Vec<CommentResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, author_id, body, created_at FROM comments WHERE question_id = $1 ORDER BY created_at ASC"
    )
    .bind(question_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("get question comments failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(
        rows.into_iter()
            .map(|r| CommentResponse {
                id: r.0,
                author_id: r.1,
                body: r.2,
                created_at: r.3,
            })
            .collect(),
    ))
}

pub async fn create_answer_comment(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<CommentResponse>), StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO comments (author_id, body, answer_id)
           VALUES ($1, $2, $3)
           RETURNING id, author_id, body, created_at"#,
    )
    .bind(auth.user_id)
    .bind(&req.body)
    .bind(answer_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("create answer comment failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CommentResponse {
            id: row.0,
            author_id: row.1,
            body: row.2,
            created_at: row.3,
        }),
    ))
}

pub async fn get_answer_comments(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
) -> Result<Json<Vec<CommentResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, author_id, body, created_at FROM comments WHERE answer_id = $1 ORDER BY created_at ASC"
    )
    .bind(answer_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("get answer comments failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(
        rows.into_iter()
            .map(|r| CommentResponse {
                id: r.0,
                author_id: r.1,
                body: r.2,
                created_at: r.3,
            })
            .collect(),
    ))
}
