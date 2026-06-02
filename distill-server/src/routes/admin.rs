use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{auth::middleware::AuthUser, AppState};

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
    _auth: AuthUser,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM config"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("get config failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    let config: HashMap<String, String> = rows.into_iter().collect();
    Ok(Json(ConfigResponse { config }))
}

pub async fn update_config(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    for (key, value) in &req.config {
        sqlx::query(
            r#"INSERT INTO config (key, value, updated_at) VALUES ($1, $2, now())
               ON CONFLICT (tenant_id, key) DO UPDATE SET value = $2, updated_at = now()"#
        )
        .bind(key)
        .bind(value)
        .execute(&state.db)
        .await
        .map_err(|e| { tracing::error!("update config failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;
    }

    get_config(State(state), _auth).await
}
