use sha2::{Digest, Sha256};
use sqlx::PgPool;

pub fn cache_key(operation: &str, input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(operation.as_bytes());
    hasher.update(b":");
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn get_cached(db: &PgPool, key: &str) -> Option<String> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT response FROM llm_cache WHERE cache_key = $1 AND expires_at > now()",
    )
    .bind(key)
    .fetch_optional(db)
    .await
    .ok()??;
    Some(row.0)
}

pub async fn store_cache(db: &PgPool, key: &str, operation: &str, response: &str, ttl_hours: i64) {
    let _ = sqlx::query(
        r#"INSERT INTO llm_cache (cache_key, operation_type, response, expires_at)
           VALUES ($1, $2, $3, now() + make_interval(hours => $4))
           ON CONFLICT (cache_key) DO UPDATE SET response = $3, expires_at = now() + make_interval(hours => $4)"#
    )
    .bind(key)
    .bind(operation)
    .bind(response)
    .bind(ttl_hours as f64)
    .execute(db)
    .await;
}
