use sqlx::PgPool;
use uuid::Uuid;

/// Generate an AI answer using hybrid retrieval for context.
pub async fn generate_ai_answer(
    db: &PgPool,
    chat_model: &str,
    question_id: Uuid,
    title: &str,
    body: &str,
) {
    if let Err(e) = do_generate(db, chat_model, question_id, title, body).await {
        tracing::error!("AI answer generation failed for {}: {:?}", question_id, e);
    }
}

async fn do_generate(
    db: &PgPool,
    chat_model: &str,
    question_id: Uuid,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use genai::chat::{ChatMessage, ChatRequest};

    let query_text = format!("{} {}", title, body);

    // Try to get the question's embedding for vector search
    let embedding: Option<pgvector::Vector> =
        sqlx::query_scalar("SELECT embedding FROM questions WHERE id = $1")
            .bind(question_id)
            .fetch_optional(db)
            .await?
            .flatten();

    // Full hybrid RRF retrieval (BM25 + vector when embedding available)
    let context_rows = if let Some(emb) = &embedding {
        sqlx::query_as::<_, (String, String, String)>(
            r#"WITH fts AS (
                 SELECT id, ROW_NUMBER() OVER (ORDER BY ts_rank(tsv, websearch_to_tsquery('english', $1)) DESC) AS rn
                 FROM questions WHERE tsv @@ websearch_to_tsquery('english', $1) AND id != $3 LIMIT 20
               ),
               vec AS (
                 SELECT id, ROW_NUMBER() OVER (ORDER BY embedding <=> $2) AS rn
                 FROM questions WHERE embedding IS NOT NULL AND id != $3 LIMIT 20
               ),
               rrf AS (
                 SELECT COALESCE(fts.id, vec.id) AS id,
                        (COALESCE(1.0/(60.0+fts.rn), 0.0) + COALESCE(1.0/(60.0+vec.rn), 0.0))::float8 AS score
                 FROM fts FULL OUTER JOIN vec ON fts.id = vec.id
               ),
               answered AS (
                 SELECT q.title AS qt, q.body AS qb, a.body AS ab,
                        ROW_NUMBER() OVER (PARTITION BY q.id ORDER BY a.created_at DESC) AS rn
                 FROM rrf JOIN questions q ON q.id = rrf.id
                 JOIN answers a ON a.question_id = q.id
                 ORDER BY rrf.score DESC
               )
               SELECT qt, qb, ab FROM answered WHERE rn = 1 LIMIT 5"#,
        )
        .bind(&query_text)
        .bind(emb)
        .bind(question_id)
        .fetch_all(db)
        .await?
    } else {
        // Fallback: BM25 only
        sqlx::query_as::<_, (String, String, String)>(
            r#"WITH bm25 AS (
                 SELECT q.id, q.title, q.body
                 FROM questions q WHERE q.tsv @@ websearch_to_tsquery('english', $1) AND q.id != $2
                 ORDER BY ts_rank(q.tsv, websearch_to_tsquery('english', $1)) DESC LIMIT 10
               ),
               answered AS (
                 SELECT b.title AS qt, b.body AS qb, a.body AS ab,
                        ROW_NUMBER() OVER (PARTITION BY b.id ORDER BY a.created_at DESC) AS rn
                 FROM bm25 b JOIN answers a ON a.question_id = b.id
               )
               SELECT qt, qb, ab FROM answered WHERE rn = 1 LIMIT 5"#,
        )
        .bind(&query_text)
        .bind(question_id)
        .fetch_all(db)
        .await?
    };

    let mut context = String::new();
    for (qt, qb, ab) in &context_rows {
        context.push_str(&format!("Q: {} - {}\nA: {}\n\n", qt, qb, ab));
    }

    let system_prompt = if context.is_empty() {
        "You are a knowledgeable assistant. Answer the question clearly and concisely.".to_string()
    } else {
        format!(
            "You are a knowledgeable assistant. Here is some relevant context from existing Q&A:\n\n{}\nAnswer the following question clearly and concisely.",
            context
        )
    };

    let client = genai::Client::default();
    let chat_req = ChatRequest::new(vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(format!("Question: {}\n\n{}", title, body)),
    ]);

    let config = crate::routes::get_config_map(db).await;
    let retries: u32 = config
        .get("llm_retry_attempts")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    let resp = crate::routes::llm_cache::retry_llm(retries, || {
        let req = chat_req.clone();
        let c = &client;
        async move { c.exec_chat(chat_model, req, None).await }
    })
    .await?;
    let answer_text = resp
        .first_text()
        .ok_or("no text in LLM response")?
        .to_string();

    let (answer_id,) = sqlx::query_as::<_, (Uuid,)>(
        r#"INSERT INTO answers (question_id, author_type, body)
           VALUES ($1, 'ai', $2) RETURNING id"#,
    )
    .bind(question_id)
    .bind(&answer_text)
    .fetch_one(db)
    .await?;

    tracing::info!("AI answer generated for question {}", question_id);

    crate::routes::contradictions::detect_contradictions(
        db,
        chat_model,
        answer_id,
        &answer_text,
        question_id,
    )
    .await;

    Ok(())
}

/// Resolve a stale answer by generating a fresh AI answer.
pub async fn resolve_stale(
    db: &PgPool,
    chat_model: &str,
    question_id: Uuid,
    title: &str,
    old_body: &str,
    stale_reason: &str,
) {
    let augmented_body = format!(
        "{}\n\n(Previous answer was marked stale: {})\nPrevious answer:\n{}",
        title, stale_reason, old_body
    );
    generate_ai_answer(db, chat_model, question_id, title, &augmented_body).await;
}
