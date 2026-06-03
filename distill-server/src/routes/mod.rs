pub mod admin;
pub mod answers;
pub mod comments;
pub mod contradictions;
pub mod graph;
pub mod links;
pub mod llm_cache;
pub mod questions;
pub mod ratings;
pub mod tags;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

struct ConfigCache {
    data: HashMap<String, String>,
    fetched_at: std::time::Instant,
}

static CONFIG_CACHE: OnceLock<RwLock<Option<ConfigCache>>> = OnceLock::new();

pub async fn get_config_map(db: &PgPool) -> HashMap<String, String> {
    let lock = CONFIG_CACHE.get_or_init(|| RwLock::new(None));

    // Check cache (30s TTL)
    {
        let guard = lock.read().await;
        if let Some(ref cached) = *guard {
            if cached.fetched_at.elapsed() < std::time::Duration::from_secs(30) {
                return cached.data.clone();
            }
        }
    }

    // Cache miss — fetch and store
    let data: HashMap<String, String> =
        sqlx::query_as::<_, (String, String)>("SELECT key, value FROM config")
            .fetch_all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();

    let mut guard = lock.write().await;
    *guard = Some(ConfigCache {
        data: data.clone(),
        fetched_at: std::time::Instant::now(),
    });
    data
}

#[derive(Deserialize)]
pub struct CursorParams {
    #[serde(default = "default_page_limit")]
    pub limit: i64,
    pub after: Option<String>, // base64-encoded "created_at,id"
}

fn default_page_limit() -> i64 {
    20
}

#[derive(Serialize)]
pub struct Paginated<T: Serialize> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

pub fn is_llm_feature_enabled(config: &HashMap<String, String>, feature_key: &str) -> bool {
    let global = config
        .get("llm_features_enabled")
        .map(|v| v == "true")
        .unwrap_or(true);
    if !global {
        return false;
    }
    config.get(feature_key).map(|v| v == "true").unwrap_or(true)
}

pub fn encode_cursor(created_at: &chrono::DateTime<chrono::Utc>, id: &uuid::Uuid) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
        "{},{}",
        created_at.to_rfc3339(),
        id
    ))
}

pub fn decode_cursor(cursor: &str) -> Option<(chrono::DateTime<chrono::Utc>, uuid::Uuid)> {
    use base64::Engine;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .ok()?;
    let s = String::from_utf8(decoded).ok()?;
    let mut parts = s.splitn(2, ',');
    let ts = parts
        .next()?
        .parse::<chrono::DateTime<chrono::Utc>>()
        .ok()?;
    let id = parts.next()?.parse::<uuid::Uuid>().ok()?;
    Some((ts, id))
}

/// Set the current tenant for RLS policies on a connection.
/// Call this before queries when multi-tenant mode is active.
pub async fn set_tenant(db: &sqlx::PgPool, tenant_id: uuid::Uuid) {
    let _: Option<String> =
        sqlx::query_scalar("SELECT set_config('app.current_tenant', $1::text, true)")
            .bind(tenant_id.to_string())
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
}
