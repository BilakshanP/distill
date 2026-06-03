use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Serialize, ToSchema)]
pub struct AnswerResponse {
    pub id: Uuid,
    pub question_id: Uuid,
    pub author_id: Option<Uuid>,
    pub author_type: String,
    pub body: String,
    pub is_stale: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct EditAnswerRequest {
    pub body: String,
    pub edit_message: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct EditHistoryEntry {
    pub id: Uuid,
    pub editor_id: Uuid,
    pub diff: String,
    pub edit_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(put, path = "/answers/{id}", request_body = EditAnswerRequest, responses((status = 200, body = AnswerResponse)), tag = "answers")]
pub async fn edit_answer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<EditAnswerRequest>,
) -> Result<Json<AnswerResponse>, StatusCode> {
    // Get current body
    let old_body: (String,) = sqlx::query_as("SELECT body FROM answers WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("edit fetch failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Compute diff
    let patch = diffy_imara::create_patch(&old_body.0, &req.body);
    let diff_text = patch.to_string();

    // Store diff
    sqlx::query(
        "INSERT INTO answer_edits (answer_id, editor_id, diff, edit_message) VALUES ($1, $2, $3, $4)"
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&diff_text)
    .bind(&req.edit_message)
    .execute(&state.db)
    .await
    .map_err(|e| { tracing::error!("edit insert failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    // Update answer body
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Option<Uuid>,
            String,
            String,
            bool,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"UPDATE answers SET body = $1, updated_at = now() WHERE id = $2
           RETURNING id, question_id, author_id, author_type, body, is_stale, created_at"#,
    )
    .bind(&req.body)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("edit update failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(AnswerResponse {
        id: row.0,
        question_id: row.1,
        author_id: row.2,
        author_type: row.3,
        body: row.4,
        is_stale: row.5,
        created_at: row.6,
    }))
}

#[utoipa::path(get, path = "/answers/{id}/history", responses((status = 200)), tag = "answers")]
pub async fn get_history(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<crate::routes::CursorParams>,
) -> Result<Json<crate::routes::Paginated<EditHistoryEntry>>, StatusCode> {
    let limit = params.limit.min(100);
    let fetch_limit = limit + 1;
    let rows = if let Some(ref cursor) = params.after {
        let (ts, cid) = crate::routes::decode_cursor(cursor).ok_or(StatusCode::BAD_REQUEST)?;
        sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, editor_id, diff, edit_message, created_at FROM answer_edits WHERE answer_id = $1 AND (created_at, id) > ($2, $3) ORDER BY created_at ASC, id ASC LIMIT $4"
        ).bind(id).bind(ts).bind(cid).bind(fetch_limit).fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, editor_id, diff, edit_message, created_at FROM answer_edits WHERE answer_id = $1 ORDER BY created_at ASC, id ASC LIMIT $2"
        ).bind(id).bind(fetch_limit).fetch_all(&state.db).await
    }.map_err(|e| { tracing::error!("history fetch failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;
    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        items
            .last()
            .map(|r| crate::routes::encode_cursor(&r.4, &r.0))
    } else {
        None
    };
    Ok(Json(crate::routes::Paginated {
        data: items
            .into_iter()
            .map(|r| EditHistoryEntry {
                id: r.0,
                editor_id: r.1,
                diff: r.2,
                edit_message: r.3,
                created_at: r.4,
            })
            .collect(),
        next_cursor,
        has_more,
    }))
}

#[utoipa::path(get, path = "/questions/{id}/answers", responses((status = 200, body = Vec<AnswerResponse>)), tag = "answers")]
pub async fn get_answers(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<Vec<AnswerResponse>>, StatusCode> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Option<Uuid>,
            String,
            String,
            bool,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"SELECT id, question_id, author_id, author_type, body, is_stale, created_at
           FROM answers WHERE question_id = $1 ORDER BY created_at ASC"#,
    )
    .bind(question_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("failed to fetch answers: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(
        rows.into_iter()
            .map(|r| AnswerResponse {
                id: r.0,
                question_id: r.1,
                author_id: r.2,
                author_type: r.3,
                body: r.4,
                is_stale: r.5,
                created_at: r.6,
            })
            .collect(),
    ))
}

pub async fn generate_ai_answer(
    db: &sqlx::PgPool,
    chat_model: &str,
    question_id: Uuid,
    title: &str,
    body: &str,
) {
    crate::services::answers::generate_ai_answer(db, chat_model, question_id, title, body).await;
}

#[derive(Deserialize, ToSchema)]
pub struct MarkStaleRequest {
    pub reason: Option<String>,
}

#[utoipa::path(post, path = "/answers/{id}/mark-stale", request_body = MarkStaleRequest, responses((status = 200, body = AnswerResponse)), tag = "answers")]
pub async fn mark_stale(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<MarkStaleRequest>,
) -> Result<Json<AnswerResponse>, StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, String, bool, chrono::DateTime<chrono::Utc>)>(
        r#"UPDATE answers SET is_stale = true, stale_reason = $1, stale_marked_by = $2, updated_at = now()
           WHERE id = $3
           RETURNING id, question_id, author_id, author_type, body, is_stale, created_at"#,
    )
    .bind(&req.reason)
    .bind(auth.user_id)
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| { tracing::error!("mark stale failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Trigger LLM auto-resolve if configured
    if let Some(chat_model) = &state.llm_chat_model {
        let config = crate::routes::get_config_map(&state.db).await;
        if config
            .get("stale_auto_resolve")
            .map(|v| v == "true")
            .unwrap_or(false)
            && crate::routes::is_llm_feature_enabled(&config, "llm_features_enabled")
        {
            let db = state.db.clone();
            let model = chat_model.clone();
            let answer_id = id;
            let body = row.4.clone();
            let reason = req.reason.clone().unwrap_or_default();

            tokio::spawn(async move {
                if let Err(e) = resolve_stale(&db, &model, answer_id, &body, &reason).await {
                    tracing::error!("stale auto-resolve failed for {}: {:?}", answer_id, e);
                }
            });
        }
    }

    Ok(Json(AnswerResponse {
        id: row.0,
        question_id: row.1,
        author_id: row.2,
        author_type: row.3,
        body: row.4,
        is_stale: row.5,
        created_at: row.6,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct DigDeeperRequest {
    pub prompt: String,
    #[serde(default = "default_true")]
    pub include_comments: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, ToSchema)]
pub struct DigDeeperResponse {
    pub id: Uuid,
    pub answer_id: Uuid,
    pub prompt: String,
    pub response: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(post, path = "/answers/{id}/dig-deeper", request_body = DigDeeperRequest, responses((status = 201, body = DigDeeperResponse)), tag = "answers")]
pub async fn dig_deeper(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<DigDeeperRequest>,
) -> Result<(StatusCode, Json<DigDeeperResponse>), StatusCode> {
    let config = crate::routes::get_config_map(&state.db).await;
    if !crate::routes::is_llm_feature_enabled(&config, "dig_deeper_enabled")
        || !crate::routes::llm_cache::check_budget(&state.db, &config).await
    {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let chat_model = state
        .llm_chat_model
        .as_deref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Get the answer and its question
    let answer_row =
        sqlx::query_as::<_, (String, Uuid)>("SELECT body, question_id FROM answers WHERE id = $1")
            .bind(answer_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("dig deeper fetch failed: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

    let question_row =
        sqlx::query_as::<_, (String, String)>("SELECT title, body FROM questions WHERE id = $1")
            .bind(answer_row.1)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("dig deeper question fetch failed: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    // Gather comments for context (optional)
    let comments_ctx = if req.include_comments {
        let comments = sqlx::query_as::<_, (String,)>(
            "SELECT body FROM comments WHERE answer_id = $1 ORDER BY created_at ASC LIMIT 20",
        )
        .bind(answer_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        if comments.is_empty() {
            String::new()
        } else {
            let c: Vec<_> = comments.iter().map(|r| r.0.as_str()).collect();
            format!("\n\nComments/discussion:\n{}", c.join("\n---\n"))
        }
    } else {
        String::new()
    };

    let cache_input = format!(
        "{}:{}:{}:{}",
        question_row.0, answer_row.0, comments_ctx, req.prompt
    );
    let cache_key = crate::routes::llm_cache::cache_key("dig_deeper", &cache_input);

    let response_text = if let Some(cached) =
        crate::routes::llm_cache::get_cached(&state.db, &cache_key).await
    {
        cached
    } else {
        use genai::chat::{ChatMessage, ChatRequest};
        let client = genai::Client::default();
        let chat_req = ChatRequest::new(vec![
            ChatMessage::system("You are a knowledgeable assistant. The user wants to explore an answer in more depth. Consider the comments/discussion context as well. Provide a detailed, helpful response."),
            ChatMessage::user(format!(
                "Original question: {} - {}\n\nCurrent answer:\n{}{}\n\nUser's follow-up:\n{}",
                question_row.0, question_row.1, answer_row.0, comments_ctx, req.prompt
            )),
        ]);

        let resp = client
            .exec_chat(chat_model, chat_req, None)
            .await
            .map_err(|e| {
                tracing::error!("dig deeper LLM failed: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let text = resp
            .first_text()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
            .to_string();

        let config = crate::routes::get_config_map(&state.db).await;
        let ttl: i64 = config
            .get("llm_cache_ttl_hours")
            .and_then(|v| v.parse().ok())
            .unwrap_or(168);
        crate::routes::llm_cache::store_cache(&state.db, &cache_key, "dig_deeper", &text, ttl)
            .await;
        text
    };

    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO deep_dives (answer_id, requester_id, prompt, response, context_sources)
           VALUES ($1, $2, $3, $4, '[]')
           RETURNING id, answer_id, prompt, response, created_at"#,
    )
    .bind(answer_id)
    .bind(auth.user_id)
    .bind(&req.prompt)
    .bind(&response_text)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("dig deeper insert failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((
        StatusCode::CREATED,
        Json(DigDeeperResponse {
            id: row.0,
            answer_id: row.1,
            prompt: row.2,
            response: row.3,
            created_at: row.4,
        }),
    ))
}

#[utoipa::path(get, path = "/answers/{id}/deep-dives", responses((status = 200)), tag = "answers")]
pub async fn get_deep_dives(
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    Query(params): Query<crate::routes::CursorParams>,
) -> Result<Json<crate::routes::Paginated<DigDeeperResponse>>, StatusCode> {
    let limit = params.limit.min(100);
    let fetch_limit = limit + 1;
    let rows = if let Some(ref cursor) = params.after {
        let (ts, cid) = crate::routes::decode_cursor(cursor).ok_or(StatusCode::BAD_REQUEST)?;
        sqlx::query_as::<_, (Uuid, Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, answer_id, prompt, response, created_at FROM deep_dives WHERE answer_id = $1 AND (created_at, id) > ($2, $3) ORDER BY created_at ASC, id ASC LIMIT $4"
        ).bind(answer_id).bind(ts).bind(cid).bind(fetch_limit).fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, (Uuid, Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, answer_id, prompt, response, created_at FROM deep_dives WHERE answer_id = $1 ORDER BY created_at ASC, id ASC LIMIT $2"
        ).bind(answer_id).bind(fetch_limit).fetch_all(&state.db).await
    }.map_err(|e| { tracing::error!("get deep dives failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;
    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        items
            .last()
            .map(|r| crate::routes::encode_cursor(&r.4, &r.0))
    } else {
        None
    };
    Ok(Json(crate::routes::Paginated {
        data: items
            .into_iter()
            .map(|r| DigDeeperResponse {
                id: r.0,
                answer_id: r.1,
                prompt: r.2,
                response: r.3,
                created_at: r.4,
            })
            .collect(),
        next_cursor,
        has_more,
    }))
}

async fn resolve_stale(
    db: &sqlx::PgPool,
    chat_model: &str,
    answer_id: uuid::Uuid,
    old_body: &str,
    stale_reason: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let question_id: (uuid::Uuid,) =
        sqlx::query_as("SELECT question_id FROM answers WHERE id = $1")
            .bind(answer_id)
            .fetch_one(db)
            .await?;
    let q_row: (String, String) = sqlx::query_as("SELECT title, body FROM questions WHERE id = $1")
        .bind(question_id.0)
        .fetch_one(db)
        .await?;
    crate::services::answers::resolve_stale(
        db,
        chat_model,
        question_id.0,
        &q_row.0,
        old_body,
        stale_reason,
    )
    .await;
    Ok(())
}
