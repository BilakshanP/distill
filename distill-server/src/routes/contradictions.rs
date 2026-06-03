use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Serialize)]
pub struct ContradictionResponse {
    pub id: Uuid,
    pub answer_id_a: Uuid,
    pub answer_id_b: Uuid,
    pub explanation: String,
    pub source: String,
    pub flagged_by: Option<Uuid>,
    pub status: String,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct ContradictionRow {
    id: Uuid,
    answer_id_a: Uuid,
    answer_id_b: Uuid,
    explanation: String,
    source: String,
    flagged_by: Option<Uuid>,
    status: String,
    detected_at: chrono::DateTime<chrono::Utc>,
}

impl From<ContradictionRow> for ContradictionResponse {
    fn from(r: ContradictionRow) -> Self {
        Self {
            id: r.id,
            answer_id_a: r.answer_id_a,
            answer_id_b: r.answer_id_b,
            explanation: r.explanation,
            source: r.source,
            flagged_by: r.flagged_by,
            status: r.status,
            detected_at: r.detected_at,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct FlagContradictionRequest {
    pub contradicts_answer_id: Uuid,
    pub explanation: String,
}

#[utoipa::path(post, path = "/answers/{id}/flag-contradiction", responses((status = 201)), tag = "contradictions")]
pub async fn flag_contradiction(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<FlagContradictionRequest>,
) -> Result<(StatusCode, Json<ContradictionResponse>), crate::error::AppError> {
    let row = sqlx::query_as::<_, ContradictionRow>(
        r#"INSERT INTO contradiction_flags (answer_id_a, answer_id_b, explanation, source, flagged_by)
           VALUES ($1, $2, $3, 'user', $4)
           ON CONFLICT (LEAST(answer_id_a, answer_id_b), GREATEST(answer_id_a, answer_id_b)) DO NOTHING
           RETURNING id, answer_id_a, answer_id_b, explanation, source, flagged_by, status, detected_at"#,
    )
    .bind(answer_id)
    .bind(req.contradicts_answer_id)
    .bind(&req.explanation)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(crate::error::AppError::Conflict("contradiction already flagged"))?;

    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(get, path = "/answers/{id}/contradictions", responses((status = 200)), tag = "contradictions")]
pub async fn get_contradictions_for_answer(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
) -> Result<Json<Vec<ContradictionResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, ContradictionRow>(
        r#"SELECT id, answer_id_a, answer_id_b, explanation, source, flagged_by, status, detected_at
           FROM contradiction_flags WHERE answer_id_a = $1 OR answer_id_b = $1
           ORDER BY detected_at DESC"#,
    )
    .bind(answer_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("get contradictions failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(
        rows.into_iter().map(ContradictionResponse::from).collect(),
    ))
}

#[utoipa::path(get, path = "/admin/contradictions", responses((status = 200)), tag = "contradictions")]
pub async fn admin_review_queue(
    State(state): State<AppState>,
    _auth: crate::auth::middleware::AdminUser,
    Query(params): Query<crate::routes::CursorParams>,
) -> Result<Json<crate::routes::Paginated<ContradictionResponse>>, StatusCode> {
    let limit = params.limit.min(100);
    let fetch_limit = limit + 1;

    let rows = if let Some(ref cursor) = params.after {
        let (ts, cid) = crate::routes::decode_cursor(cursor).ok_or(StatusCode::BAD_REQUEST)?;
        sqlx::query_as::<_, ContradictionRow>(
            r#"SELECT id, answer_id_a, answer_id_b, explanation, source, flagged_by, status, detected_at
               FROM contradiction_flags WHERE status = 'pending' AND (detected_at, id) > ($1, $2)
               ORDER BY detected_at ASC, id ASC LIMIT $3"#,
        )
        .bind(ts).bind(cid).bind(fetch_limit)
        .fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, ContradictionRow>(
            r#"SELECT id, answer_id_a, answer_id_b, explanation, source, flagged_by, status, detected_at
               FROM contradiction_flags WHERE status = 'pending'
               ORDER BY detected_at ASC, id ASC LIMIT $1"#,
        )
        .bind(fetch_limit)
        .fetch_all(&state.db).await
    }.map_err(|e| { tracing::error!("admin queue failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        items
            .last()
            .map(|r| crate::routes::encode_cursor(&r.detected_at, &r.id))
    } else {
        None
    };

    Ok(Json(crate::routes::Paginated {
        data: items.into_iter().map(ContradictionResponse::from).collect(),
        next_cursor,
        has_more,
    }))
}

/// Auto-detect contradictions for a newly created answer
pub async fn detect_contradictions(
    db: &sqlx::PgPool,
    chat_model: &str,
    answer_id: Uuid,
    answer_body: &str,
    question_id: Uuid,
) {
    let config = crate::routes::get_config_map(db).await;
    if !crate::routes::is_llm_feature_enabled(&config, "auto_contradiction_detection") {
        return;
    }
    if let Err(e) = do_detect(db, chat_model, answer_id, answer_body, question_id).await {
        tracing::error!("contradiction detection failed for {}: {:?}", answer_id, e);
    }
}

async fn do_detect(
    db: &sqlx::PgPool,
    chat_model: &str,
    answer_id: Uuid,
    answer_body: &str,
    question_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use genai::chat::{ChatMessage, ChatRequest};

    // Semantic prefilter: find candidate answers via embedding similarity of parent questions
    let emb_row: Option<(pgvector::Vector, Option<String>)> = sqlx::query_as(
        "SELECT embedding, embedding_model FROM questions WHERE id = $1 AND embedding IS NOT NULL",
    )
    .bind(question_id)
    .fetch_optional(db)
    .await?;

    let other_answers = if let Some((emb, model)) = emb_row {
        let model = model.unwrap_or_default();
        // Find answers to the most semantically similar questions (same embedding model)
        sqlx::query_as::<_, (Uuid, String)>(
            r#"SELECT a.id, a.body FROM answers a
               JOIN questions q ON q.id = a.question_id
               WHERE a.id != $1 AND q.embedding IS NOT NULL AND q.embedding_model = $3
               ORDER BY q.embedding <=> $2
               LIMIT 5"#,
        )
        .bind(answer_id)
        .bind(emb)
        .bind(model)
        .fetch_all(db)
        .await?
    } else {
        // Fallback: same-question answers
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, body FROM answers WHERE question_id = $1 AND id != $2 ORDER BY created_at DESC LIMIT 5",
        )
        .bind(question_id)
        .bind(answer_id)
        .fetch_all(db)
        .await?
    };

    if other_answers.is_empty() {
        return Ok(());
    }

    let client = genai::Client::default();
    let config = crate::routes::get_config_map(db).await;
    let ttl: i64 = config
        .get("llm_cache_ttl_hours")
        .and_then(|v| v.parse().ok())
        .unwrap_or(168);

    let retries: u32 = config
        .get("llm_retry_attempts")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    for (other_id, other_body) in &other_answers {
        let cache_input = format!("{}:{}", answer_body, other_body);
        let cache_key = crate::routes::llm_cache::cache_key("contradiction", &cache_input);

        let text = if let Some(cached) = crate::routes::llm_cache::get_cached(db, &cache_key).await
        {
            cached
        } else {
            let chat_req = ChatRequest::new(vec![
                ChatMessage::system("You are a contradiction detector. Compare two answers and determine if they contradict each other. Reply ONLY with 'NO' if they don't contradict, or a brief explanation of the contradiction if they do."),
                ChatMessage::user(format!("Answer A:\n{}\n\nAnswer B:\n{}", answer_body, other_body)),
            ]);

            let resp = crate::routes::llm_cache::retry_llm(retries, || {
                let req = chat_req.clone();
                let c = &client;
                async move { c.exec_chat(chat_model, req, None).await }
            })
            .await?;
            let result = resp.first_text().unwrap_or("NO").trim().to_string();
            crate::routes::llm_cache::store_cache(db, &cache_key, "contradiction", &result, ttl)
                .await;
            result
        };

        if text != "NO" && !text.to_lowercase().starts_with("no") {
            sqlx::query(
                r#"INSERT INTO contradiction_flags (answer_id_a, answer_id_b, explanation, source)
                   VALUES ($1, $2, $3, 'auto')
                   ON CONFLICT (LEAST(answer_id_a, answer_id_b), GREATEST(answer_id_a, answer_id_b)) DO NOTHING"#,
            )
            .bind(answer_id)
            .bind(other_id)
            .bind(&text)
            .execute(db)
            .await?;

            tracing::info!(
                "contradiction detected between {} and {}",
                answer_id,
                other_id
            );
        }
    }

    Ok(())
}
