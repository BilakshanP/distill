use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::middleware::AuthUser, AppState};

#[derive(Serialize, ToSchema)]
pub struct WikiAnswerResponse {
    pub id: Uuid,
    pub question_id: Uuid,
    pub body: String,
    pub author_id: Option<Uuid>,
    pub last_editor_id: Option<Uuid>,
    pub is_stale: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct EditWikiAnswerRequest {
    pub body: String,
    pub edit_message: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct RevisionSummary {
    pub id: Uuid,
    pub editor_id: Uuid,
    pub edit_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct RevisionDetail {
    pub id: Uuid,
    pub editor_id: Uuid,
    pub body: String,
    pub diff: String,
    pub edit_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_wiki_answer(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<WikiAnswerResponse>, StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, Option<Uuid>, Option<Uuid>, bool, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, question_id, body, author_id, last_editor_id, is_stale, created_at, updated_at FROM wiki_answers WHERE question_id = $1"
    )
    .bind(question_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match row {
        Some(r) => Ok(Json(WikiAnswerResponse {
            id: r.0,
            question_id: r.1,
            body: r.2,
            author_id: r.3,
            last_editor_id: r.4,
            is_stale: r.5,
            created_at: r.6,
            updated_at: r.7,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn edit_wiki_answer(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<EditWikiAnswerRequest>,
) -> Result<Json<WikiAnswerResponse>, StatusCode> {
    // Fetch old body before upsert
    let old_body =
        sqlx::query_scalar::<_, String>("SELECT body FROM wiki_answers WHERE question_id = $1")
            .bind(question_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();

    // Upsert wiki answer
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, Option<Uuid>, Option<Uuid>, bool, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        r#"INSERT INTO wiki_answers (question_id, body, author_id, last_editor_id)
           VALUES ($1, $2, $3, $3)
           ON CONFLICT (question_id) DO UPDATE SET
             body = EXCLUDED.body,
             last_editor_id = $3,
             updated_at = now()
           RETURNING id, question_id, body, author_id, last_editor_id, is_stale, created_at, updated_at"#
    )
    .bind(question_id)
    .bind(&req.body)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| { tracing::error!("wiki answer upsert: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    // Compute diff and store revision with full body
    let diff = diffy_imara::create_patch(&old_body, &req.body).to_string();

    sqlx::query("INSERT INTO wiki_answer_edits (wiki_answer_id, editor_id, body, diff, edit_message) VALUES ($1, $2, $3, $4, $5)")
        .bind(row.0)
        .bind(auth.user_id)
        .bind(&req.body)
        .bind(&diff)
        .bind(&req.edit_message)
        .execute(&state.db)
        .await
        .ok();

    Ok(Json(WikiAnswerResponse {
        id: row.0,
        question_id: row.1,
        body: row.2,
        author_id: row.3,
        last_editor_id: row.4,
        is_stale: row.5,
        created_at: row.6,
        updated_at: row.7,
    }))
}

/// List revision summaries (for the history page sidebar)
pub async fn get_wiki_answer_history(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<Vec<RevisionSummary>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Option<String>, chrono::DateTime<chrono::Utc>)>(
        r#"SELECT e.id, e.editor_id, e.edit_message, e.created_at
           FROM wiki_answer_edits e
           JOIN wiki_answers w ON w.id = e.wiki_answer_id
           WHERE w.question_id = $1
           ORDER BY e.created_at DESC"#,
    )
    .bind(question_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        rows.into_iter()
            .map(|r| RevisionSummary {
                id: r.0,
                editor_id: r.1,
                edit_message: r.2,
                created_at: r.3,
            })
            .collect(),
    ))
}

/// Get a single revision with its diff
pub async fn get_revision(
    State(state): State<AppState>,
    Path(revision_id): Path<Uuid>,
) -> Result<Json<RevisionDetail>, StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, editor_id, body, diff, edit_message, created_at FROM wiki_answer_edits WHERE id = $1"
    )
    .bind(revision_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match row {
        Some(r) => Ok(Json(RevisionDetail {
            id: r.0,
            editor_id: r.1,
            body: r.2,
            diff: r.3,
            edit_message: r.4,
            created_at: r.5,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}
