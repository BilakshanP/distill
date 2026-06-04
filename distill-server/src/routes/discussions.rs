use axum::{extract::Path, extract::Query, extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Serialize, ToSchema)]
pub struct DiscussionResponse {
    pub id: Uuid,
    pub question_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub author_id: Uuid,
    pub author_name: String,
    pub author_role: String,
    pub author_avatar: Option<String>,
    pub body: String,
    pub depth: i32,
    pub is_deleted: bool,
    pub score: i64,
    pub user_vote: Option<i16>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateDiscussionRequest {
    pub body: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct DiscussionParams {
    pub parent_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub after: Option<String>,
}

fn default_limit() -> i64 {
    50
}

pub async fn create_discussion(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<CreateDiscussionRequest>,
) -> Result<(StatusCode, Json<DiscussionResponse>), StatusCode> {
    let depth = if let Some(pid) = req.parent_id {
        let d: Option<(i32,)> = sqlx::query_as("SELECT depth FROM discussions WHERE id = $1")
            .bind(pid)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        d.map(|r| r.0 + 1).unwrap_or(0)
    } else {
        0
    };

    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Option<Uuid>,
            Uuid,
            String,
            i32,
            bool,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"INSERT INTO discussions (question_id, parent_id, author_id, body, depth)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, question_id, parent_id, author_id, body, depth, is_deleted, created_at"#,
    )
    .bind(question_id)
    .bind(req.parent_id)
    .bind(auth.user_id)
    .bind(&req.body)
    .bind(depth)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("create discussion: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Fetch author info
    let author: (String, String, Option<String>) =
        sqlx::query_as("SELECT display_name, role, avatar_url FROM users WHERE id = $1")
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(DiscussionResponse {
            id: row.0,
            question_id: row.1,
            parent_id: row.2,
            author_id: row.3,
            body: row.4,
            depth: row.5,
            is_deleted: row.6,
            score: 0,
            user_vote: None,
            author_name: author.0,
            author_role: author.1,
            author_avatar: author.2,
            created_at: row.7,
        }),
    ))
}

pub async fn list_discussions(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    Query(params): Query<DiscussionParams>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<DiscussionResponse>>, StatusCode> {
    // Extract user_id from token if present (optional auth)
    let user_id = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .and_then(|token| crate::auth::jwt::validate_token(token, &state.jwt_secret).ok())
        .map(|claims| claims.sub)
        .unwrap_or(Uuid::nil());

    let rows = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, Uuid, String, i32, bool, chrono::DateTime<chrono::Utc>, i64, Option<i16>, String, String, Option<String>)>(
        r#"SELECT d.id, d.question_id, d.parent_id, d.author_id, d.body, d.depth, d.is_deleted, d.created_at,
                  COALESCE(SUM(v.direction), 0) AS score,
                  (SELECT direction FROM discussion_votes WHERE discussion_id = d.id AND user_id = $3) AS user_vote,
                  u.display_name, u.role, u.avatar_url
           FROM discussions d
           JOIN users u ON u.id = d.author_id
           LEFT JOIN discussion_votes v ON v.discussion_id = d.id
           WHERE d.question_id = $1 AND ($2::uuid IS NULL OR d.parent_id = $2)
           GROUP BY d.id, u.display_name, u.role, u.avatar_url
           ORDER BY COALESCE(SUM(v.direction), 0) DESC, d.created_at ASC
           LIMIT $4"#
    )
    .bind(question_id)
    .bind(params.parent_id)
    .bind(user_id)
    .bind(params.limit.min(100))
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("list discussions: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(
        rows.into_iter()
            .map(|r| DiscussionResponse {
                id: r.0,
                question_id: r.1,
                parent_id: r.2,
                author_id: r.3,
                body: if r.6 { "[deleted]".into() } else { r.4 },
                depth: r.5,
                is_deleted: r.6,
                score: r.8,
                user_vote: r.9,
                author_name: r.10,
                author_role: r.11,
                author_avatar: r.12,
                created_at: r.7,
            })
            .collect(),
    ))
}
