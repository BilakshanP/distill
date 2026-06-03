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

    sqlx::query("UPDATE questions SET embedding = $1, embedding_model = $2, embedding_version = $3 WHERE id = $4")
        .bind(vector)
        .bind(model)
        .bind(crate::EMBEDDING_VERSION)
        .bind(question_id)
        .execute(db)
        .await?;

    tracing::info!("embedding generated for question {}", question_id);
    Ok(())
}
