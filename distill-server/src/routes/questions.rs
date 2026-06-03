use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Deserialize, ToSchema)]
pub struct CreateQuestionRequest {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_metadata")]
    pub metadata: serde_json::Value,
}

fn default_metadata() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(sqlx::FromRow)]
struct SearchResultRow {
    id: Uuid,
    title: String,
    body: String,
    tags: Vec<String>,
    score: f64,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct QuestionListRow {
    id: Uuid,
    author_id: Uuid,
    title: String,
    body: String,
    original_query: String,
    tags: Vec<String>,
    metadata: serde_json::Value,
    status: String,
    has_embedding: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct QuestionResponse {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub body: String,
    pub original_query: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub status: String,
    pub has_embedding: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(post, path = "/questions", request_body = CreateQuestionRequest, responses((status = 201, body = QuestionResponse)), tag = "questions")]
pub async fn create_question(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateQuestionRequest>,
) -> Result<(StatusCode, Json<QuestionResponse>), StatusCode> {
    let original_query = format!("{} {}", req.title, req.body);

    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            Vec<String>,
            serde_json::Value,
            String,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"INSERT INTO questions (author_id, title, body, original_query, tags, metadata)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, title, body, original_query, tags, metadata, status, created_at"#,
    )
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&req.body)
    .bind(&original_query)
    .bind(&req.tags)
    .bind(&req.metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("failed to insert question: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Optionally generate embedding in background
    if let Some(model) = &state.llm_embedding_model {
        let _ = crate::jobs::enqueue(
            &state.db,
            &crate::jobs::JobPayload::GenerateEmbedding {
                question_id: row.0,
                text: original_query.clone(),
                model: model.clone(),
            },
        )
        .await;
    }

    // Generate AI answer in background (respects answer_mode config)
    if let Some(chat_model) = &state.llm_chat_model {
        let config = crate::routes::get_config_map(&state.db).await;
        let answer_mode = config
            .get("answer_mode")
            .map(|s| s.as_str())
            .unwrap_or("ai-first");
        if answer_mode != "community-only"
            && crate::routes::llm_cache::check_budget(&state.db, &config).await
        {
            let _ = crate::jobs::enqueue(
                &state.db,
                &crate::jobs::JobPayload::GenerateAiAnswer {
                    question_id: row.0,
                    title: req.title.clone(),
                    body: req.body.clone(),
                    model: chat_model.clone(),
                },
            )
            .await;
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(QuestionResponse {
            id: row.0,
            author_id: auth.user_id,
            title: row.1,
            body: row.2,
            original_query: row.3,
            tags: row.4,
            metadata: row.5,
            status: row.6,
            has_embedding: false, // async, not ready yet
            created_at: row.7,
        }),
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct PreviewRequest {
    pub title: String,
    pub body: String,
}

#[derive(Serialize, ToSchema)]
pub struct PreviewResponse {
    pub matches: Vec<SearchResult>,
    pub rephrased: Option<String>,
}

#[utoipa::path(post, path = "/questions/preview", request_body = PreviewRequest, responses((status = 200, body = PreviewResponse)), tag = "questions")]
pub async fn preview_question(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>, StatusCode> {
    let query = format!("{} {}", req.title, req.body);

    // Find matching questions using hybrid search
    let query_embedding = if let Some(model) = &state.llm_embedding_model {
        let client = genai::Client::default();
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            client.embed(model.as_str(), &query, None),
        )
        .await
        {
            Ok(Ok(r)) => Some(pgvector::Vector::from(r.embeddings[0].vector.clone())),
            Ok(Err(e)) => {
                tracing::error!("preview embedding failed: {:?}", e);
                None
            }
            Err(_) => {
                tracing::warn!("preview embedding timed out, falling back to keyword-only");
                None
            }
        }
    } else {
        None
    };

    let matches = if let Some(embedding) = &query_embedding {
        sqlx::query_as::<_, SearchResultRow>(
            r#"
            WITH fts AS (
                SELECT id, ROW_NUMBER() OVER (ORDER BY ts_rank(tsv, websearch_to_tsquery('english', $1)) DESC) AS rn
                FROM questions WHERE tsv @@ websearch_to_tsquery('english', $1) LIMIT 50
            ),
            vec AS (
                SELECT id, ROW_NUMBER() OVER (ORDER BY embedding <=> $2) AS rn
                FROM questions WHERE embedding IS NOT NULL AND embedding_model = $3 LIMIT 50
            ),
            rrf AS (
                SELECT COALESCE(fts.id, vec.id) AS id,
                       (COALESCE(1.0 / (60.0 + fts.rn), 0.0) + COALESCE(1.0 / (60.0 + vec.rn), 0.0))::float8 AS score
                FROM fts FULL OUTER JOIN vec ON fts.id = vec.id
            )
            SELECT q.id, q.title, q.body, q.tags, rrf.score, q.created_at
            FROM rrf JOIN questions q ON q.id = rrf.id
            ORDER BY rrf.score DESC LIMIT 5
            "#,
        )
        .bind(&query)
        .bind(embedding)
        .bind(state.llm_embedding_model.as_deref().unwrap_or(""))
        .fetch_all(&state.db)
        .await
        .map_err(|e| { tracing::error!("preview search query failed: {:?}", e); })
        .unwrap_or_default()
    } else {
        sqlx::query_as::<_, SearchResultRow>(
            r#"SELECT id, title, body, tags, ts_rank(tsv, websearch_to_tsquery('english', $1))::float8 AS score, created_at
               FROM questions WHERE tsv @@ websearch_to_tsquery('english', $1)
               ORDER BY score DESC LIMIT 5"#,
        )
        .bind(&query)
        .fetch_all(&state.db)
        .await
        .map_err(|e| { tracing::error!("preview keyword search failed: {:?}", e); })
        .unwrap_or_default()
    };

    // Rephrase via LLM (with cache)
    let rephrased = if let Some(model) = &state.llm_chat_model {
        let config = crate::routes::get_config_map(&state.db).await;
        if !crate::routes::is_llm_feature_enabled(&config, "rephrase_enabled")
            || !crate::routes::llm_cache::check_budget(&state.db, &config).await
        {
            None
        } else {
            let cache_input = format!("{}:{}", req.title, req.body);
            let key = crate::routes::llm_cache::cache_key("rephrase", &cache_input);

            if let Some(cached) = crate::routes::llm_cache::get_cached(&state.db, &key).await {
                Some(cached)
            } else {
                use genai::chat::{ChatMessage, ChatRequest};
                let client = genai::Client::default();
                let chat_req = ChatRequest::new(vec![
                ChatMessage::system("You are a helpful assistant that rephrases questions to be clearer and more searchable. Return ONLY the rephrased question, nothing else."),
                ChatMessage::user(format!("Rephrase this question:\nTitle: {}\nBody: {}", req.title, req.body)),
            ]);
                match client.exec_chat(model.as_str(), chat_req, None).await {
                    Ok(resp) => {
                        let text = resp.first_text().map(|s| s.to_string());
                        if let Some(ref t) = text {
                            let config = crate::routes::get_config_map(&state.db).await;
                            let ttl: i64 = config
                                .get("llm_cache_ttl_hours")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(168);
                            crate::routes::llm_cache::store_cache(
                                &state.db, &key, "rephrase", t, ttl,
                            )
                            .await;
                        }
                        text
                    }
                    Err(e) => {
                        tracing::warn!("rephrase failed: {}", e);
                        None
                    }
                }
            }
        }
    } else {
        None
    };

    Ok(Json(PreviewResponse {
        matches: matches
            .into_iter()
            .map(|r| SearchResult {
                id: r.id,
                title: r.title,
                body: r.body,
                tags: r.tags,
                score: r.score,
                created_at: r.created_at,
            })
            .collect(),
        rephrased,
    }))
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    /// Comma-separated tags to filter by
    pub tags: Option<String>,
}

fn default_limit() -> i64 {
    20
}

#[derive(Serialize, ToSchema)]
pub struct SearchResult {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub score: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(get, path = "/questions/search", params(("q" = String, Query, description = "Search query")), responses((status = 200, body = Vec<SearchResult>)), tag = "questions", security(()))]
pub async fn search_questions(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    let limit = params.limit.min(100);
    let k: f64 = 60.0;

    let config = crate::routes::get_config_map(&state.db).await;
    let search_mode = config
        .get("search_mode")
        .map(|s| s.as_str())
        .unwrap_or("hybrid");

    // Generate embedding for the query if model is configured and search_mode is hybrid
    let query_embedding = if search_mode == "hybrid" {
        if let Some(model) = &state.llm_embedding_model {
            let client = genai::Client::default();
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                client.embed(model.as_str(), &params.q, None),
            )
            .await
            {
                Ok(Ok(resp)) => Some(pgvector::Vector::from(resp.embeddings[0].vector.clone())),
                Ok(Err(e)) => {
                    tracing::warn!("embedding generation for search query failed: {}", e);
                    None
                }
                Err(_) => {
                    tracing::warn!("embedding generation timed out, falling back to keyword-only");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let results = if let Some(embedding) = query_embedding {
        // Hybrid search: BM25 + vector with RRF
        sqlx::query_as::<_, SearchResultRow>(
            r#"
            WITH fts AS (
                SELECT id, ts_rank(tsv, websearch_to_tsquery('english', $1)) AS rank,
                       ROW_NUMBER() OVER (ORDER BY ts_rank(tsv, websearch_to_tsquery('english', $1)) DESC) AS rn
                FROM questions
                WHERE tsv @@ websearch_to_tsquery('english', $1)
                LIMIT 100
            ),
            vec AS (
                SELECT id, 1 - (embedding <=> $2) AS rank,
                       ROW_NUMBER() OVER (ORDER BY embedding <=> $2) AS rn
                FROM questions
                WHERE embedding IS NOT NULL AND embedding_model = $6
                LIMIT 100
            ),
            rrf AS (
                SELECT COALESCE(fts.id, vec.id) AS id,
                       (COALESCE(1.0 / ($3 + fts.rn), 0.0) + COALESCE(1.0 / ($3 + vec.rn), 0.0))::float8 AS score
                FROM fts FULL OUTER JOIN vec ON fts.id = vec.id
            )
            SELECT q.id, q.title, q.body, q.tags, rrf.score, q.created_at
            FROM rrf
            JOIN questions q ON q.id = rrf.id
            ORDER BY rrf.score DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(&params.q)
        .bind(&embedding)
        .bind(k)
        .bind(limit)
        .bind(params.offset)
        .bind(state.llm_embedding_model.as_deref().unwrap_or(""))
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("hybrid search failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        // Keyword-only fallback
        let tag_filter: Vec<String> = params
            .tags
            .as_deref()
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        if tag_filter.is_empty() {
            sqlx::query_as::<_, SearchResultRow>(
                r#"
                SELECT id, title, body, tags, ts_rank(tsv, websearch_to_tsquery('english', $1))::float8 AS score, created_at
                FROM questions
                WHERE tsv @@ websearch_to_tsquery('english', $1)
                ORDER BY score DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&params.q)
            .bind(limit)
            .bind(params.offset)
            .fetch_all(&state.db)
            .await
        } else {
            sqlx::query_as::<_, SearchResultRow>(
                r#"
                SELECT id, title, body, tags, ts_rank(tsv, websearch_to_tsquery('english', $1))::float8 AS score, created_at
                FROM questions
                WHERE tsv @@ websearch_to_tsquery('english', $1) AND tags @> $4
                ORDER BY score DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&params.q)
            .bind(limit)
            .bind(params.offset)
            .bind(&tag_filter)
            .fetch_all(&state.db)
            .await
        }
        .map_err(|e| {
            tracing::error!("keyword search failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    Ok(Json(
        results
            .into_iter()
            .map(|r| SearchResult {
                id: r.id,
                title: r.title,
                body: r.body,
                tags: r.tags,
                score: r.score,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

#[utoipa::path(get, path = "/questions", responses((status = 200, description = "Paginated list of questions")), tag = "questions", security(()))]
pub async fn list_questions(
    State(state): State<AppState>,
    Query(params): Query<crate::routes::CursorParams>,
) -> Result<Json<crate::routes::Paginated<QuestionResponse>>, StatusCode> {
    let limit = params.limit.min(100);
    let fetch_limit = limit + 1;

    let rows = if let Some(ref cursor) = params.after {
        let (ts, cid) = crate::routes::decode_cursor(cursor).ok_or(StatusCode::BAD_REQUEST)?;
        sqlx::query_as::<_, QuestionListRow>(
            r#"SELECT id, author_id, title, body, original_query, tags, metadata, status, embedding IS NOT NULL AS has_embedding, created_at
               FROM questions WHERE (created_at, id) < ($1, $2)
               ORDER BY created_at DESC, id DESC LIMIT $3"#,
        )
        .bind(ts).bind(cid).bind(fetch_limit)
        .fetch_all(&state.db).await
    } else {
        sqlx::query_as::<_, QuestionListRow>(
            r#"SELECT id, author_id, title, body, original_query, tags, metadata, status, embedding IS NOT NULL AS has_embedding, created_at
               FROM questions ORDER BY created_at DESC, id DESC LIMIT $1"#,
        )
        .bind(fetch_limit)
        .fetch_all(&state.db).await
    }.map_err(|e| { tracing::error!("list questions failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    let has_more = rows.len() as i64 > limit;
    let items: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = items
        .last()
        .map(|r| crate::routes::encode_cursor(&r.created_at, &r.id));

    Ok(Json(crate::routes::Paginated {
        data: items
            .into_iter()
            .map(|r| QuestionResponse {
                id: r.id,
                author_id: r.author_id,
                title: r.title,
                body: r.body,
                original_query: r.original_query,
                tags: r.tags,
                metadata: r.metadata,
                status: r.status,
                has_embedding: r.has_embedding,
                created_at: r.created_at,
            })
            .collect(),
        next_cursor: if has_more { next_cursor } else { None },
        has_more,
    }))
}

#[utoipa::path(get, path = "/questions/{id}", responses((status = 200, body = QuestionResponse)), tag = "questions", security(()))]
pub async fn get_question(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<QuestionResponse>, crate::error::AppError> {
    let row = sqlx::query_as::<_, QuestionListRow>(
        r#"SELECT id, author_id, title, body, original_query, tags, metadata, status, embedding IS NOT NULL AS has_embedding, created_at
           FROM questions WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(crate::error::AppError::NotFound("question not found"))?;

    Ok(Json(QuestionResponse {
        id: row.id,
        author_id: row.author_id,
        title: row.title,
        body: row.body,
        original_query: row.original_query,
        tags: row.tags,
        metadata: row.metadata,
        status: row.status,
        has_embedding: row.has_embedding,
        created_at: row.created_at,
    }))
}
