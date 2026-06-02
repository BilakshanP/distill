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

/// Retry an async LLM call up to `max_retries` times on transient errors.
pub async fn retry_llm<F, Fut, T, E>(max_retries: u32, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                attempt += 1;
                if attempt > max_retries {
                    return Err(e);
                }
                let msg = e.to_string();
                if msg.contains("503") || msg.contains("429") || msg.contains("UNAVAILABLE") {
                    tracing::warn!(
                        "LLM transient error (attempt {}/{}): {}",
                        attempt,
                        max_retries,
                        msg
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
}
