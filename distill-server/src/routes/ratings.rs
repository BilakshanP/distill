use axum::{
    extract::{Path, State},
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
}

#[derive(Serialize, ToSchema)]
pub struct RatingResponse {
    pub id: Uuid,
    pub wiki_answer_id: Uuid,
    pub rater_id: Uuid,
    pub score: i32,
    pub scale_type: String,
    pub comment: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// POST /wiki-answers/{id}/ratings — rate a wiki answer (upsert)
#[utoipa::path(post, path = "/wiki-answers/{id}/ratings", request_body = CreateRatingRequest, responses((status = 201, body = RatingResponse)), tag = "ratings")]
pub async fn create_rating(
    State(state): State<AppState>,
    Path(wiki_answer_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<CreateRatingRequest>,
) -> Result<(StatusCode, Json<RatingResponse>), StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, i32, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO answer_ratings (wiki_answer_id, rater_id, score, comment)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (wiki_answer_id, rater_id) DO UPDATE SET score = EXCLUDED.score, comment = EXCLUDED.comment
           RETURNING id, wiki_answer_id, rater_id, score, scale_type, comment, created_at"#,
    )
    .bind(wiki_answer_id)
    .bind(auth.user_id)
    .bind(req.score)
    .bind(&req.comment)
    .fetch_one(&state.db)
    .await
    .map_err(|e| { tracing::error!("rating insert failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok((
        StatusCode::CREATED,
        Json(RatingResponse {
            id: row.0,
            wiki_answer_id: row.1,
            rater_id: row.2,
            score: row.3,
            scale_type: row.4,
            comment: row.5,
            created_at: row.6,
        }),
    ))
}

/// GET /wiki-answers/{id}/ratings
pub async fn get_ratings(
    State(state): State<AppState>,
    Path(wiki_answer_id): Path<Uuid>,
) -> Result<Json<Vec<RatingResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, i32, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, wiki_answer_id, rater_id, score, scale_type, comment, created_at FROM answer_ratings WHERE wiki_answer_id = $1 ORDER BY created_at DESC",
    )
    .bind(wiki_answer_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        rows.into_iter()
            .map(|r| RatingResponse {
                id: r.0,
                wiki_answer_id: r.1,
                rater_id: r.2,
                score: r.3,
                scale_type: r.4,
                comment: r.5,
                created_at: r.6,
            })
            .collect(),
    ))
}

/// DELETE /wiki-answers/{id}/ratings/mine — remove your rating
pub async fn delete_rating(
    State(state): State<AppState>,
    Path(wiki_answer_id): Path<Uuid>,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM answer_ratings WHERE wiki_answer_id = $1 AND rater_id = $2")
        .bind(wiki_answer_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
