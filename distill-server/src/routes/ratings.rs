use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Deserialize)]
pub struct CreateRatingRequest {
    pub score: i32,
    pub comment: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub rater_original_query: Option<String>,
}

#[derive(Serialize)]
pub struct RatingResponse {
    pub id: Uuid,
    pub answer_id: Uuid,
    pub rater_id: Uuid,
    pub score: i32,
    pub scale_type: String,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    pub rater_original_query: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_rating(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<CreateRatingRequest>,
) -> Result<(StatusCode, Json<RatingResponse>), StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, i32, String, Option<String>, Vec<String>, Option<String>, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO ratings (answer_id, rater_id, score, comment, tags, rater_original_query)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (answer_id, rater_id) DO UPDATE SET
             score = EXCLUDED.score, comment = EXCLUDED.comment,
             tags = EXCLUDED.tags, rater_original_query = EXCLUDED.rater_original_query
           RETURNING id, answer_id, rater_id, score, scale_type, comment, tags, rater_original_query, created_at"#,
    )
    .bind(answer_id)
    .bind(auth.user_id)
    .bind(req.score)
    .bind(&req.comment)
    .bind(&req.tags)
    .bind(&req.rater_original_query)
    .fetch_one(&state.db)
    .await
    .map_err(|e| { tracing::error!("rating insert failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok((StatusCode::CREATED, Json(RatingResponse {
        id: row.0, answer_id: row.1, rater_id: row.2, score: row.3,
        scale_type: row.4, comment: row.5, tags: row.6,
        rater_original_query: row.7, created_at: row.8,
    })))
}

pub async fn get_ratings(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    Query(params): Query<crate::routes::CursorParams>,
) -> Result<Json<crate::routes::Paginated<RatingResponse>>, StatusCode> {
    let limit = params.limit.min(100);
    let fetch_limit = limit + 1; // fetch one extra to detect has_more

    let rows = if let Some(ref cursor) = params.after {
        let (ts, cid) = crate::routes::decode_cursor(cursor).ok_or(StatusCode::BAD_REQUEST)?;
        sqlx::query_as::<_, (Uuid, Uuid, Uuid, i32, String, Option<String>, Vec<String>, Option<String>, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, answer_id, rater_id, score, scale_type, comment, tags, rater_original_query, created_at FROM ratings WHERE answer_id = $1 AND (created_at, id) < ($2, $3) ORDER BY created_at DESC, id DESC LIMIT $4"
        )
        .bind(answer_id).bind(ts).bind(cid).bind(fetch_limit)
        .fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, (Uuid, Uuid, Uuid, i32, String, Option<String>, Vec<String>, Option<String>, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, answer_id, rater_id, score, scale_type, comment, tags, rater_original_query, created_at FROM ratings WHERE answer_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2"
        )
        .bind(answer_id).bind(fetch_limit)
        .fetch_all(&state.db).await
    }.map_err(|e| { tracing::error!("ratings fetch failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        items.last().map(|r| crate::routes::encode_cursor(&r.8, &r.0))
    } else {
        None
    };

    Ok(Json(crate::routes::Paginated {
        data: items.into_iter().map(|r| RatingResponse {
            id: r.0, answer_id: r.1, rater_id: r.2, score: r.3,
            scale_type: r.4, comment: r.5, tags: r.6,
            rater_original_query: r.7, created_at: r.8,
        }).collect(),
        next_cursor,
        has_more,
    }))
}
