use axum::{
    extract::{Path, Query, State},
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
    Query(params): Query<crate::routes::CursorParams>,
) -> Result<Json<crate::routes::Paginated<CommentResponse>>, StatusCode> {
    let limit = params.limit.min(100);
    let fetch_limit = limit + 1;
    let rows = if let Some(ref cursor) = params.after {
        let (ts, cid) = crate::routes::decode_cursor(cursor).ok_or(StatusCode::BAD_REQUEST)?;
        sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, author_id, body, created_at FROM comments WHERE question_id = $1 AND (created_at, id) > ($2, $3) ORDER BY created_at ASC, id ASC LIMIT $4"
        ).bind(question_id).bind(ts).bind(cid).bind(fetch_limit).fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, author_id, body, created_at FROM comments WHERE question_id = $1 ORDER BY created_at ASC, id ASC LIMIT $2"
        ).bind(question_id).bind(fetch_limit).fetch_all(&state.db).await
    }.map_err(|e| { tracing::error!("get question comments failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;
    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        items
            .last()
            .map(|r| crate::routes::encode_cursor(&r.3, &r.0))
    } else {
        None
    };
    Ok(Json(crate::routes::Paginated {
        data: items
            .into_iter()
            .map(|r| CommentResponse {
                id: r.0,
                author_id: r.1,
                body: r.2,
                created_at: r.3,
            })
            .collect(),
        next_cursor,
        has_more,
    }))
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
    Query(params): Query<crate::routes::CursorParams>,
) -> Result<Json<crate::routes::Paginated<CommentResponse>>, StatusCode> {
    let limit = params.limit.min(100);
    let fetch_limit = limit + 1;
    let rows = if let Some(ref cursor) = params.after {
        let (ts, cid) = crate::routes::decode_cursor(cursor).ok_or(StatusCode::BAD_REQUEST)?;
        sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, author_id, body, created_at FROM comments WHERE answer_id = $1 AND (created_at, id) > ($2, $3) ORDER BY created_at ASC, id ASC LIMIT $4"
        ).bind(answer_id).bind(ts).bind(cid).bind(fetch_limit).fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, author_id, body, created_at FROM comments WHERE answer_id = $1 ORDER BY created_at ASC, id ASC LIMIT $2"
        ).bind(answer_id).bind(fetch_limit).fetch_all(&state.db).await
    }.map_err(|e| { tracing::error!("get answer comments failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;
    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        items
            .last()
            .map(|r| crate::routes::encode_cursor(&r.3, &r.0))
    } else {
        None
    };
    Ok(Json(crate::routes::Paginated {
        data: items
            .into_iter()
            .map(|r| CommentResponse {
                id: r.0,
                author_id: r.1,
                body: r.2,
                created_at: r.3,
            })
            .collect(),
        next_cursor,
        has_more,
    }))
}
