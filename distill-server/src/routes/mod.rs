pub mod admin;
pub mod answers;
pub mod contradictions;
pub mod graph;
pub mod questions;
pub mod ratings;

use std::collections::HashMap;
use sqlx::PgPool;

pub async fn get_config_map(db: &PgPool) -> HashMap<String, String> {
    sqlx::query_as::<_, (String, String)>("SELECT key, value FROM config")
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect()
}
