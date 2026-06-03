use sqlx::PgPool;
use uuid::Uuid;

/// Generate and store embedding for a question.
pub async fn generate_embedding(
    db: &PgPool,
    model: &str,
    question_id: Uuid,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use pgvector::Vector;

    let client = genai::Client::default();
    let resp = client.embed(model, text, None).await?;
    let vector = Vector::from(resp.embeddings[0].vector.clone());

    sqlx::query("UPDATE questions SET embedding = $1 WHERE id = $2")
        .bind(vector)
        .bind(question_id)
        .execute(db)
        .await?;

    tracing::info!("embedding generated for question {}", question_id);
    Ok(())
}
