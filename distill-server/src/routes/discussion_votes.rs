use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Deserialize)]
pub struct VoteRequest {
    pub direction: i16, // 1 or -1
}

#[derive(Serialize)]
pub struct VoteResponse {
    pub score: i64,
    pub user_vote: Option<i16>,
}

/// POST /discussions/:id/vote — toggle vote
/// Same direction = remove vote, different = update
pub async fn vote_discussion(
    State(state): State<AppState>,
    Path(discussion_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<VoteRequest>,
) -> Result<Json<VoteResponse>, StatusCode> {
    if req.direction != 1 && req.direction != -1 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check existing vote
    let existing: Option<(i16,)> = sqlx::query_as(
        "SELECT direction FROM discussion_votes WHERE discussion_id = $1 AND user_id = $2",
    )
    .bind(discussion_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match existing {
        Some((dir,)) if dir == req.direction => {
            // Same vote = remove
            sqlx::query("DELETE FROM discussion_votes WHERE discussion_id = $1 AND user_id = $2")
                .bind(discussion_id)
                .bind(auth.user_id)
                .execute(&state.db)
                .await
                .ok();
        }
        Some(_) => {
            // Different vote = update
            sqlx::query("UPDATE discussion_votes SET direction = $3 WHERE discussion_id = $1 AND user_id = $2")
                .bind(discussion_id).bind(auth.user_id).bind(req.direction)
                .execute(&state.db).await.ok();
        }
        None => {
            // New vote
            sqlx::query("INSERT INTO discussion_votes (discussion_id, user_id, direction) VALUES ($1, $2, $3)")
                .bind(discussion_id).bind(auth.user_id).bind(req.direction)
                .execute(&state.db).await.ok();
        }
    }

    // Return updated score
    let score: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(direction), 0) FROM discussion_votes WHERE discussion_id = $1",
    )
    .bind(discussion_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_vote: Option<(i16,)> = sqlx::query_as(
        "SELECT direction FROM discussion_votes WHERE discussion_id = $1 AND user_id = $2",
    )
    .bind(discussion_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    Ok(Json(VoteResponse {
        score: score.0,
        user_vote: user_vote.map(|v| v.0),
    }))
}
