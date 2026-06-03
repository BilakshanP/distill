use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Deserialize, ToSchema)]
pub struct CreateRatingRequest {
    pub score: i32,
    pub comment: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub rater_original_query: Option<String>,
    /// When rater_context_visibility=optional, rater chooses whether to include context.
    /// Defaults to true.
    #[serde(default = "default_true")]
    pub include_context: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, ToSchema)]
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

#[derive(sqlx::FromRow)]
struct RatingRow {
    id: Uuid,
    answer_id: Uuid,
    rater_id: Uuid,
    score: i32,
    scale_type: String,
    comment: Option<String>,
    tags: Vec<String>,
    rater_original_query: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<RatingRow> for RatingResponse {
    fn from(r: RatingRow) -> Self {
        Self {
            id: r.id,
            answer_id: r.answer_id,
            rater_id: r.rater_id,
            score: r.score,
            scale_type: r.scale_type,
            comment: r.comment,
            tags: r.tags,
            rater_original_query: r.rater_original_query,
            created_at: r.created_at,
        }
    }
}

#[utoipa::path(post, path = "/answers/{id}/ratings", request_body = CreateRatingRequest, responses((status = 201, body = RatingResponse)), tag = "ratings")]
pub async fn create_rating(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<CreateRatingRequest>,
) -> Result<(StatusCode, Json<RatingResponse>), StatusCode> {
    let config = crate::routes::get_config_map(&state.db).await;
    let visibility = crate::config_enums::RaterContextVisibility::from_config(&config);

    // Determine what context to store based on config
    let (comment, query) = match visibility {
        crate::config_enums::RaterContextVisibility::Never => (None, None),
        crate::config_enums::RaterContextVisibility::Always => {
            (req.comment.clone(), req.rater_original_query.clone())
        }
        crate::config_enums::RaterContextVisibility::Optional => {
            if req.include_context {
                (req.comment.clone(), req.rater_original_query.clone())
            } else {
                (None, None)
            }
        }
    };

    let row = sqlx::query_as::<_, RatingRow>(
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
    .bind(&comment)
    .bind(&req.tags)
    .bind(&query)
    .fetch_one(&state.db)
    .await
    .map_err(|e| { tracing::error!("rating insert failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(put, path = "/answers/{id}/ratings/redact", responses((status = 204)), tag = "ratings")]
pub async fn redact_rating(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query(
        "UPDATE ratings SET rater_original_query = NULL, comment = NULL WHERE answer_id = $1 AND rater_id = $2"
    )
    .bind(answer_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| { tracing::error!("redact rating failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/answers/{id}/ratings", responses((status = 200)), tag = "ratings", security(()))]
pub async fn get_ratings(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    Query(params): Query<crate::routes::CursorParams>,
) -> Result<Json<crate::routes::Paginated<RatingResponse>>, StatusCode> {
    let limit = params.limit.min(100);
    let fetch_limit = limit + 1; // fetch one extra to detect has_more

    let rows = if let Some(ref cursor) = params.after {
        let (ts, cid) = crate::routes::decode_cursor(cursor).ok_or(StatusCode::BAD_REQUEST)?;
        sqlx::query_as::<_, RatingRow>(
            "SELECT id, answer_id, rater_id, score, scale_type, comment, tags, rater_original_query, created_at FROM ratings WHERE answer_id = $1 AND (created_at, id) < ($2, $3) ORDER BY created_at DESC, id DESC LIMIT $4"
        )
        .bind(answer_id).bind(ts).bind(cid).bind(fetch_limit)
        .fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, RatingRow>(
            "SELECT id, answer_id, rater_id, score, scale_type, comment, tags, rater_original_query, created_at FROM ratings WHERE answer_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2"
        )
        .bind(answer_id).bind(fetch_limit)
        .fetch_all(&state.db).await
    }.map_err(|e| { tracing::error!("ratings fetch failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        items
            .last()
            .map(|r| crate::routes::encode_cursor(&r.created_at, &r.id))
    } else {
        None
    };

    Ok(Json(crate::routes::Paginated {
        data: items.into_iter().map(RatingResponse::from).collect(),
        next_cursor,
        has_more,
    }))
}
