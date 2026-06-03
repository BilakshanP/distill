use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LinkRequest {
    pub target_question_id: Uuid,
    #[serde(default = "default_link_type")]
    pub link_type: String,
}

fn default_link_type() -> String {
    "related".into()
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct LinkResponse {
    pub id: Uuid,
    pub question_id_a: Uuid,
    pub question_id_b: Uuid,
    pub link_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(post, path = "/questions/{id}/link", request_body = LinkRequest, responses((status = 201, body = LinkResponse)), tag = "links")]
pub async fn link_questions(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<LinkRequest>,
) -> Result<(StatusCode, Json<LinkResponse>), StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO question_links (question_id_a, question_id_b, link_type, created_by)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (question_id_a, question_id_b) DO UPDATE SET link_type = EXCLUDED.link_type
           RETURNING id, question_id_a, question_id_b, link_type, created_at"#,
    )
    .bind(question_id)
    .bind(req.target_question_id)
    .bind(&req.link_type)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("link questions failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((
        StatusCode::CREATED,
        Json(LinkResponse {
            id: row.0,
            question_id_a: row.1,
            question_id_b: row.2,
            link_type: row.3,
            created_at: row.4,
        }),
    ))
}
