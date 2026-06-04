use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Serialize, ToSchema)]
pub struct AnswerResponse {
    pub id: Uuid,
    pub question_id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub author_role: String,
    pub body: String,
    pub is_accepted: bool,
    pub rating_avg: Option<f64>,
    pub rating_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAnswerRequest {
    pub body: String,
}

pub async fn create_answer(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<CreateAnswerRequest>,
) -> Result<(StatusCode, Json<AnswerResponse>), StatusCode> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            String,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"INSERT INTO answers (question_id, author_id, body)
           VALUES ($1, $2, $3)
           RETURNING id, question_id, author_id, body, created_at, updated_at"#,
    )
    .bind(question_id)
    .bind(auth.user_id)
    .bind(&req.body)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("create answer: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let author: (String, String) =
        sqlx::query_as("SELECT display_name, role FROM users WHERE id = $1")
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(AnswerResponse {
            id: row.0,
            question_id: row.1,
            author_id: row.2,
            author_name: author.0,
            author_role: author.1,
            body: row.3,
            is_accepted: false,
            rating_avg: None,
            rating_count: 0,
            created_at: row.4,
            updated_at: row.5,
        }),
    ))
}

pub async fn list_answers(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<Vec<AnswerResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, String, String, bool, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, i64, Option<f64>)>(
        r#"SELECT a.id, a.question_id, a.author_id, u.display_name, u.role, a.body, a.is_accepted, a.created_at, a.updated_at,
                  COALESCE(r.cnt, 0), r.avg_score
           FROM answers a
           JOIN users u ON u.id = a.author_id
           LEFT JOIN LATERAL (
               SELECT COUNT(*) AS cnt, AVG(score)::float8 AS avg_score
               FROM individual_answer_ratings WHERE answer_id = a.id
           ) r ON true
           WHERE a.question_id = $1
           ORDER BY a.is_accepted DESC, COALESCE(r.avg_score, 0) DESC, a.created_at ASC"#,
    )
    .bind(question_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("list answers: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(
        rows.into_iter()
            .map(|r| AnswerResponse {
                id: r.0,
                question_id: r.1,
                author_id: r.2,
                author_name: r.3,
                author_role: r.4,
                body: r.5,
                is_accepted: r.6,
                created_at: r.7,
                updated_at: r.8,
                rating_count: r.9,
                rating_avg: r.10,
            })
            .collect(),
    ))
}

/// POST /answers/{id}/ratings — rate an individual answer
pub async fn rate_answer(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<RateRequest>,
) -> Result<Json<RateResponse>, StatusCode> {
    if req.score < 1 || req.score > 5 {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query(
        r#"INSERT INTO individual_answer_ratings (answer_id, rater_id, score)
           VALUES ($1, $2, $3)
           ON CONFLICT (answer_id, rater_id) DO UPDATE SET score = EXCLUDED.score"#,
    )
    .bind(answer_id)
    .bind(auth.user_id)
    .bind(req.score)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats: (i64, Option<f64>) = sqlx::query_as(
        "SELECT COUNT(*), AVG(score)::float8 FROM individual_answer_ratings WHERE answer_id = $1",
    )
    .bind(answer_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, None));

    Ok(Json(RateResponse {
        rating_count: stats.0,
        rating_avg: stats.1,
        your_score: Some(req.score),
    }))
}

/// DELETE /answers/{id}/ratings/mine
pub async fn delete_answer_rating(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM individual_answer_ratings WHERE answer_id = $1 AND rater_id = $2")
        .bind(answer_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct RateRequest {
    pub score: i32,
}

#[derive(Serialize)]
pub struct RateResponse {
    pub rating_count: i64,
    pub rating_avg: Option<f64>,
    pub your_score: Option<i32>,
}
