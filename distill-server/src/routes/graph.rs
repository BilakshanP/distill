use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Serialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Serialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub node_type: String, // "question", "answer", "tag"
    pub label: String,
    pub size: i64,
}

#[derive(Serialize)]
pub struct GraphEdge {
    pub source: Uuid,
    pub target: Uuid,
    pub edge_type: String, // "has_answer", "similar", "contradicts", "tagged"
    pub weight: f64,
}

#[derive(Deserialize)]
pub struct GraphParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

pub async fn get_graph(
    State(state): State<AppState>,
    Query(params): Query<GraphParams>,
) -> Result<Json<GraphResponse>, StatusCode> {
    let limit = params.limit.min(500);

    // Get question nodes
    let questions = sqlx::query_as::<_, (Uuid, String, i64)>(
        r#"SELECT q.id, q.title, COUNT(a.id) AS answer_count
           FROM questions q LEFT JOIN answers a ON a.question_id = q.id
           GROUP BY q.id ORDER BY q.created_at DESC LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("graph questions failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    let mut nodes: Vec<GraphNode> = questions.iter().map(|q| GraphNode {
        id: q.0, node_type: "question".into(), label: q.1.clone(), size: q.2 + 1,
    }).collect();

    // Get answer nodes and question->answer edges
    let answers = sqlx::query_as::<_, (Uuid, Uuid, i64)>(
        r#"SELECT a.id, a.question_id, COUNT(r.id) AS rating_count
           FROM answers a LEFT JOIN ratings r ON r.answer_id = a.id
           WHERE a.question_id = ANY($1)
           GROUP BY a.id"#,
    )
    .bind(&questions.iter().map(|q| q.0).collect::<Vec<_>>())
    .fetch_all(&state.db)
    .await
    .map_err(|e| { tracing::error!("graph answers failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    let mut edges: Vec<GraphEdge> = Vec::new();

    for a in &answers {
        nodes.push(GraphNode {
            id: a.0, node_type: "answer".into(), label: format!("Answer"), size: a.2 + 1,
        });
        edges.push(GraphEdge {
            source: a.1, target: a.0, edge_type: "has_answer".into(), weight: 1.0,
        });
    }

    // Get contradiction edges
    let contradictions = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT answer_id_a, answer_id_b FROM contradiction_flags WHERE status != 'dismissed'"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for c in &contradictions {
        edges.push(GraphEdge {
            source: c.0, target: c.1, edge_type: "contradicts".into(), weight: 1.0,
        });
    }

    Ok(Json(GraphResponse { nodes, edges }))
}

pub async fn get_node_neighborhood(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GraphResponse>, StatusCode> {
    // Get the focal question and its answers
    let question = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, title FROM questions WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| { tracing::error!("graph node failed: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?
    .ok_or(StatusCode::NOT_FOUND)?;

    let mut nodes = vec![GraphNode {
        id: question.0, node_type: "question".into(), label: question.1, size: 3,
    }];
    let mut edges: Vec<GraphEdge> = Vec::new();

    // Answers to this question
    let answers = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, LEFT(body, 50) FROM answers WHERE question_id = $1"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for a in &answers {
        nodes.push(GraphNode { id: a.0, node_type: "answer".into(), label: a.1.clone(), size: 1 });
        edges.push(GraphEdge { source: id, target: a.0, edge_type: "has_answer".into(), weight: 1.0 });
    }

    // Similar questions (via vector similarity if embedding exists)
    let similar = sqlx::query_as::<_, (Uuid, String, f64)>(
        r#"SELECT q2.id, q2.title, 1 - (q1.embedding <=> q2.embedding) AS similarity
           FROM questions q1, questions q2
           WHERE q1.id = $1 AND q2.id != $1
             AND q1.embedding IS NOT NULL AND q2.embedding IS NOT NULL
           ORDER BY q1.embedding <=> q2.embedding LIMIT 5"#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for s in &similar {
        nodes.push(GraphNode { id: s.0, node_type: "question".into(), label: s.1.clone(), size: 2 });
        edges.push(GraphEdge { source: id, target: s.0, edge_type: "similar".into(), weight: s.2 });
    }

    Ok(Json(GraphResponse { nodes, edges }))
}
