use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub score: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviewResponse {
    pub matches: Vec<SearchResult>,
    pub rephrased: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnswerResponse {
    pub id: Uuid,
    pub question_id: Uuid,
    pub author_id: Option<Uuid>,
    pub author_type: String,
    pub body: String,
    pub is_stale: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RatingResponse {
    pub id: Uuid,
    pub answer_id: Uuid,
    pub rater_id: Uuid,
    pub score: i32,
    pub scale_type: String,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    pub rater_original_query: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DigDeeperResponse {
    pub id: Uuid,
    pub answer_id: Uuid,
    pub prompt: String,
    pub response: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub node_type: String,
    pub label: String,
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: Uuid,
    pub target: Uuid,
    pub edge_type: String,
    pub weight: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditHistoryEntry {
    pub id: Uuid,
    pub editor_id: Uuid,
    pub diff: String,
    pub edit_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkResponse {
    pub id: Uuid,
    pub question_id_a: Uuid,
    pub question_id_b: Uuid,
    pub link_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentResponse {
    pub id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub config: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ReEmbedResponse {
    pub enqueued: i64,
}

#[derive(Debug, Deserialize)]
pub struct TenantResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct JobResponse {
    pub id: uuid::Uuid,
    pub job_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}
