pub mod admin;
pub mod answers;
pub mod contradictions;
pub mod graph;
pub mod questions;
pub mod ratings;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub async fn get_config_map(db: &PgPool) -> HashMap<String, String> {
    sqlx::query_as::<_, (String, String)>("SELECT key, value FROM config")
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect()
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

pub fn encode_cursor(created_at: &chrono::DateTime<chrono::Utc>, id: &uuid::Uuid) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!("{},{}", created_at.to_rfc3339(), id))
}

pub fn decode_cursor(cursor: &str) -> Option<(chrono::DateTime<chrono::Utc>, uuid::Uuid)> {
    use base64::Engine;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(cursor).ok()?;
    let s = String::from_utf8(decoded).ok()?;
    let mut parts = s.splitn(2, ',');
    let ts = parts.next()?.parse::<chrono::DateTime<chrono::Utc>>().ok()?;
    let id = parts.next()?.parse::<uuid::Uuid>().ok()?;
    Some((ts, id))
}
