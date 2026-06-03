//! Retrieval evaluation harness.
//!
//! Measures precision@k and MRR against a JSONL eval set.
//! Each line: {"query": "...", "relevant_ids": ["uuid", ...]}
//!
//! Usage: cargo run --bin eval -- --eval-file eval_set.jsonl

use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use std::io::BufRead;

#[derive(Deserialize)]
struct EvalCase {
    query: String,
    relevant_ids: Vec<uuid::Uuid>,
}

#[derive(Default)]
struct Metrics {
    total: usize,
    precision_at_5_sum: f64,
    mrr_sum: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let eval_file = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "distill-server/tests/fixtures/eval_set.jsonl".to_string());

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://distill:distill@localhost:5432/distill".into());
    let db = PgPoolOptions::new().connect(&db_url).await?;

    let file = std::fs::File::open(&eval_file)?;
    let reader = std::io::BufReader::new(file);

    let mut metrics = Metrics::default();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let case: EvalCase = serde_json::from_str(&line)?;
        let results = search_bm25(&db, &case.query).await;

        let relevant: std::collections::HashSet<_> = case.relevant_ids.into_iter().collect();
        let k = 5;

        // Precision@k
        let hits = results
            .iter()
            .take(k)
            .filter(|id| relevant.contains(id))
            .count();
        let precision = hits as f64 / k as f64;

        // Mean Reciprocal Rank
        let mrr = results
            .iter()
            .enumerate()
            .find(|(_, id)| relevant.contains(id))
            .map(|(i, _)| 1.0 / (i as f64 + 1.0))
            .unwrap_or(0.0);

        metrics.total += 1;
        metrics.precision_at_5_sum += precision;
        metrics.mrr_sum += mrr;
    }

    if metrics.total == 0 {
        eprintln!("No eval cases found in {}", eval_file);
        return Ok(());
    }

    println!("=== Retrieval Evaluation ===");
    println!("Cases:        {}", metrics.total);
    println!(
        "Precision@5:  {:.3}",
        metrics.precision_at_5_sum / metrics.total as f64
    );
    println!(
        "MRR:          {:.3}",
        metrics.mrr_sum / metrics.total as f64
    );

    Ok(())
}

async fn search_bm25(db: &sqlx::PgPool, query: &str) -> Vec<uuid::Uuid> {
    sqlx::query_scalar(
        r#"WITH fts AS (
             SELECT id, ROW_NUMBER() OVER (ORDER BY ts_rank(tsv, websearch_to_tsquery('english', $1)) DESC) AS rn
             FROM questions WHERE tsv @@ websearch_to_tsquery('english', $1) LIMIT 50
           ),
           vec AS (
             SELECT id, ROW_NUMBER() OVER (ORDER BY embedding <=> (
               SELECT embedding FROM questions WHERE original_query = $1 LIMIT 1
             )) AS rn
             FROM questions WHERE embedding IS NOT NULL LIMIT 50
           ),
           rrf AS (
             SELECT COALESCE(fts.id, vec.id) AS id,
                    (COALESCE(1.0/(60.0+fts.rn), 0.0) + COALESCE(1.0/(60.0+vec.rn), 0.0))::float8 AS score
             FROM fts FULL OUTER JOIN vec ON fts.id = vec.id
           )
           SELECT id FROM rrf ORDER BY score DESC LIMIT 10"#,
    )
    .bind(query)
    .fetch_all(db)
    .await
    .unwrap_or_default()
}
