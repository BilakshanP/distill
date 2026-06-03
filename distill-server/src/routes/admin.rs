use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{auth::middleware::AdminUser, AppState};

#[derive(Serialize)]
pub struct ConfigResponse {
    pub config: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub config: HashMap<String, String>,
}

pub async fn get_config(
    State(state): State<AppState>,
    _auth: AdminUser,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let rows = sqlx::query_as::<_, (String, String)>("SELECT key, value FROM config")
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("get config failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let config: HashMap<String, String> = rows.into_iter().collect();
    Ok(Json(ConfigResponse { config }))
}

pub async fn update_config(
    State(state): State<AppState>,
    _auth: AdminUser,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    for (key, value) in &req.config {
        sqlx::query(
            r#"INSERT INTO config (key, value, updated_at) VALUES ($1, $2, now())
               ON CONFLICT (tenant_id, key) DO UPDATE SET value = $2, updated_at = now()"#,
        )
        .bind(key)
        .bind(value)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("update config failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    get_config(State(state), _auth).await
}

#[derive(Deserialize)]
pub struct SetUserQuotaRequest {
    pub user_id: uuid::Uuid,
    pub monthly_quota: Option<i32>, // None = unlimited
}

pub async fn set_user_quota(
    State(state): State<AppState>,
    _auth: AdminUser,
    Json(req): Json<SetUserQuotaRequest>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("UPDATE users SET llm_quota_monthly = $1 WHERE id = $2")
        .bind(req.monthly_quota)
        .bind(req.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct ReEmbedResponse {
    pub enqueued: i64,
}

/// Enqueue re-embedding jobs for questions with outdated embedding_version.
pub async fn re_embed(
    State(state): State<AppState>,
    _auth: AdminUser,
) -> Result<Json<ReEmbedResponse>, StatusCode> {
    let model = state
        .llm_embedding_model
        .as_deref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let rows = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, original_query FROM questions WHERE embedding_version < $1",
    )
    .bind(crate::EMBEDDING_VERSION)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let count = rows.len() as i64;
    for (qid, text) in rows {
        let _ = crate::jobs::enqueue(
            &state.db,
            &crate::jobs::JobPayload::GenerateEmbedding {
                question_id: qid,
                text,
                model: model.to_string(),
            },
        )
        .await;
    }

    Ok(Json(ReEmbedResponse { enqueued: count }))
}

pub async fn promote_user(
    State(state): State<AppState>,
    _auth: AdminUser,
    axum::extract::Path(user_id): axum::extract::Path<uuid::Uuid>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("UPDATE users SET role = 'admin' WHERE id = $1 AND role != 'admin'")
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct JobResponse {
    pub id: uuid::Uuid,
    pub job_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct JobsParams {
    #[serde(default)]
    pub status: Option<String>, // pending, running, completed, failed
    #[serde(default = "default_jobs_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_jobs_limit() -> i64 {
    50
}

pub async fn list_jobs(
    State(state): State<AppState>,
    _auth: AdminUser,
    axum::extract::Query(params): axum::extract::Query<JobsParams>,
) -> Result<Json<Vec<JobResponse>>, StatusCode> {
    let limit = params.limit.min(200);

    let rows = if let Some(ref status) = params.status {
        sqlx::query_as::<_, (uuid::Uuid, String, String, i32, i32, Option<String>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT id, job_type, status, attempts, max_attempts, error, created_at, started_at, completed_at FROM jobs WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(status).bind(limit).bind(params.offset)
        .fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, (uuid::Uuid, String, String, i32, i32, Option<String>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT id, job_type, status, attempts, max_attempts, error, created_at, started_at, completed_at FROM jobs ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit).bind(params.offset)
        .fetch_all(&state.db).await
    }.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        rows.into_iter()
            .map(|r| JobResponse {
                id: r.0,
                job_type: r.1,
                status: r.2,
                attempts: r.3,
                max_attempts: r.4,
                error: r.5,
                created_at: r.6,
                started_at: r.7,
                completed_at: r.8,
            })
            .collect(),
    ))
}
