//! Contradiction detection evaluation harness.
//!
//! Measures precision and recall against a labeled eval set.
//! Each JSONL line: {"answer_a": "...", "answer_b": "...", "contradicts": true/false}
//!
//! Usage: cargo run --bin eval_contradictions -- --eval-file contradiction_eval.jsonl

use genai::chat::{ChatMessage, ChatRequest};
use serde::Deserialize;
use std::io::BufRead;

#[derive(Deserialize)]
struct EvalCase {
    answer_a: String,
    answer_b: String,
    contradicts: bool,
}

#[derive(Default)]
struct Metrics {
    total: usize,
    true_positives: usize,
    false_positives: usize,
    true_negatives: usize,
    false_negatives: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let eval_file = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "distill-server/tests/fixtures/contradiction_eval.jsonl".to_string());
    let model = std::env::var("LLM_CHAT_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".into());

    let file = std::fs::File::open(&eval_file)?;
    let reader = std::io::BufReader::new(file);
    let client = genai::Client::default();

    let mut metrics = Metrics::default();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let case: EvalCase = serde_json::from_str(&line)?;

        let chat_req = ChatRequest::new(vec![
            ChatMessage::system("You are a contradiction detector. Compare two answers and determine if they contradict each other. Reply with EXACTLY 'NONE' if they don't contradict, or 'CONTRADICTION: <brief explanation>' if they do."),
            ChatMessage::user(format!("Answer A:\n{}\n\nAnswer B:\n{}", case.answer_a, case.answer_b)),
        ]);

        let resp = client.exec_chat(&model, chat_req, None).await?;
        let text = resp.first_text().unwrap_or("NO").trim().to_string();
        let predicted = text.starts_with("CONTRADICTION:");

        metrics.total += 1;
        match (predicted, case.contradicts) {
            (true, true) => metrics.true_positives += 1,
            (true, false) => metrics.false_positives += 1,
            (false, true) => metrics.false_negatives += 1,
            (false, false) => metrics.true_negatives += 1,
        }

        println!(
            "[{}] predicted={} actual={} | {}",
            if predicted == case.contradicts {
                "✓"
            } else {
                "✗"
            },
            predicted,
            case.contradicts,
            if predicted { &text } else { "NO" }
        );
    }

    if metrics.total == 0 {
        eprintln!("No eval cases found in {}", eval_file);
        return Ok(());
    }

    let precision = if metrics.true_positives + metrics.false_positives > 0 {
        metrics.true_positives as f64 / (metrics.true_positives + metrics.false_positives) as f64
    } else {
        0.0
    };
    let recall = if metrics.true_positives + metrics.false_negatives > 0 {
        metrics.true_positives as f64 / (metrics.true_positives + metrics.false_negatives) as f64
    } else {
        0.0
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    println!("\n=== Contradiction Detection Evaluation ===");
    println!("Cases:      {}", metrics.total);
    println!("Precision:  {:.3}", precision);
    println!("Recall:     {:.3}", recall);
    println!("F1:         {:.3}", f1);
    println!(
        "Accuracy:   {:.3}",
        (metrics.true_positives + metrics.true_negatives) as f64 / metrics.total as f64
    );

    Ok(())
}
