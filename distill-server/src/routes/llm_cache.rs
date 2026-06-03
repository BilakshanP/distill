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

/// Check if the monthly token budget allows a new LLM call.
/// Returns true if budget is unlimited or not yet exhausted.
pub async fn check_budget(db: &PgPool, config: &std::collections::HashMap<String, String>) -> bool {
    let budget_str = config
        .get("token_budget_monthly")
        .map(|s| s.as_str())
        .unwrap_or("");
    if budget_str.is_empty() {
        return true; // unlimited
    }
    let budget: i64 = match budget_str.parse() {
        Ok(v) => v,
        Err(_) => return true,
    };

    let used: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(LENGTH(response)), 0) FROM llm_cache WHERE created_at >= date_trunc('month', now())"
    )
    .fetch_one(db)
    .await
    .unwrap_or((0,));

    used.0 < budget
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

/// Check if a user has remaining LLM quota this month.
/// Returns true if no per-user quota is set, or if usage is within limit.
pub async fn check_user_quota(db: &PgPool, user_id: uuid::Uuid) -> bool {
    let quota: Option<i32> =
        sqlx::query_scalar("SELECT llm_quota_monthly FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .flatten();

    let Some(limit) = quota else { return true }; // No quota set = unlimited

    let used: i32 = sqlx::query_scalar(
        "SELECT request_count FROM user_llm_usage WHERE user_id = $1 AND month = date_trunc('month', now())::date"
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    used < limit
}

/// Increment a user's LLM usage count for this month.
pub async fn increment_user_usage(db: &PgPool, user_id: uuid::Uuid) {
    sqlx::query(
        r#"INSERT INTO user_llm_usage (user_id, month, request_count)
           VALUES ($1, date_trunc('month', now())::date, 1)
           ON CONFLICT (user_id, month) DO UPDATE SET request_count = user_llm_usage.request_count + 1"#,
    )
    .bind(user_id)
    .execute(db)
    .await
    .ok();
}
