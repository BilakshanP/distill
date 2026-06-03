use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

fn rand_jitter() -> f64 {
    // Simple pseudo-random 0.0..1.0 using time nanos
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JobPayload {
    GenerateEmbedding {
        question_id: Uuid,
        text: String,
        model: String,
    },
    GenerateAiAnswer {
        question_id: Uuid,
        title: String,
        body: String,
        model: String,
    },
    ResolveStale {
        question_id: Uuid,
        title: String,
        old_body: String,
        reason: String,
        model: String,
    },
}

/// Enqueue a job for background processing.
pub async fn enqueue(db: &PgPool, payload: &JobPayload) -> Result<Uuid, sqlx::Error> {
    let job_type = match payload {
        JobPayload::GenerateEmbedding { .. } => "generate_embedding",
        JobPayload::GenerateAiAnswer { .. } => "generate_ai_answer",
        JobPayload::ResolveStale { .. } => "resolve_stale",
    };
    let json = serde_json::to_value(payload).unwrap();
    let (id,): (Uuid,) =
        sqlx::query_as("INSERT INTO jobs (job_type, payload) VALUES ($1, $2) RETURNING id")
            .bind(job_type)
            .bind(json)
            .fetch_one(db)
            .await?;
    Ok(id)
}

/// Poll and process pending jobs. Run this in a background loop.
pub async fn process_pending(db: &PgPool) {
    let jobs = sqlx::query_as::<_, (Uuid, String, serde_json::Value, i32, i32)>(
        r#"UPDATE jobs SET status = 'running', started_at = now(), attempts = attempts + 1
           WHERE id IN (
             SELECT id FROM jobs WHERE status = 'pending' AND next_attempt_at <= now()
             ORDER BY created_at LIMIT 5
             FOR UPDATE SKIP LOCKED
           )
           RETURNING id, job_type, payload, attempts, max_attempts"#,
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    for (id, _job_type, payload, attempts, max_attempts) in jobs {
        let result = match serde_json::from_value::<JobPayload>(payload) {
            Ok(p) => execute_job(db, &p).await,
            Err(e) => Err(format!("deserialize failed: {}", e).into()),
        };

        match result {
            Ok(()) => {
                sqlx::query(
                    "UPDATE jobs SET status = 'completed', completed_at = now() WHERE id = $1",
                )
                .bind(id)
                .execute(db)
                .await
                .ok();
            }
            Err(e) => {
                let new_status = if attempts >= max_attempts {
                    "failed"
                } else {
                    "pending"
                };
                // Exponential backoff: base 2 with jitter, capped at 60s
                let base = 2i64.pow(attempts as u32).min(60);
                let jitter = (base as f64 * 0.5 * rand_jitter()) as i64;
                let backoff_secs = base + jitter;
                sqlx::query("UPDATE jobs SET status = $1, error = $2, next_attempt_at = now() + make_interval(secs => $4) WHERE id = $3")
                    .bind(new_status)
                    .bind(e.to_string())
                    .bind(id)
                    .bind(backoff_secs as f64)
                    .execute(db)
                    .await
                    .ok();
            }
        }
    }
}

async fn execute_job(
    db: &PgPool,
    payload: &JobPayload,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match payload {
        JobPayload::GenerateEmbedding {
            question_id,
            text,
            model,
        } => crate::services::questions::generate_embedding(db, model, *question_id, text).await,
        JobPayload::GenerateAiAnswer {
            question_id,
            title,
            body,
            model,
        } => {
            crate::services::answers::generate_ai_answer(db, model, *question_id, title, body)
                .await;
            Ok(())
        }
        JobPayload::ResolveStale {
            question_id,
            title,
            old_body,
            reason,
            model,
        } => {
            crate::services::answers::resolve_stale(
                db,
                model,
                *question_id,
                title,
                old_body,
                reason,
            )
            .await;
            Ok(())
        }
    }
}

/// Spawn the background worker loop.
pub fn spawn_worker(db: PgPool) {
    tokio::spawn(async move {
        loop {
            process_pending(&db).await;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}
