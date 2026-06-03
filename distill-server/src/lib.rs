pub mod auth;
pub mod config;
pub mod error;
pub mod routes;
pub mod services;

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;
#[cfg(debug_assertions)]
use utoipa::OpenApi;
#[cfg(debug_assertions)]
use utoipa_swagger_ui::SwaggerUi;

use auth::middleware::AuthUser;

#[cfg(debug_assertions)]
#[derive(OpenApi)]
#[openapi(
    paths(
        routes::questions::create_question,
        routes::questions::get_question,
        routes::questions::search_questions,
        routes::questions::preview_question,
        routes::answers::get_answers,
        routes::answers::edit_answer,
        routes::answers::get_history,
        routes::answers::mark_stale,
        routes::answers::dig_deeper,
        routes::answers::get_deep_dives,
        routes::ratings::create_rating,
        routes::ratings::get_ratings,
        routes::ratings::redact_rating,
    ),
    components(schemas(
        routes::questions::CreateQuestionRequest,
        routes::questions::QuestionResponse,
        routes::questions::SearchResult,
        routes::questions::PreviewRequest,
        routes::questions::PreviewResponse,
        routes::answers::AnswerResponse,
        routes::answers::EditAnswerRequest,
        routes::answers::EditHistoryEntry,
        routes::answers::MarkStaleRequest,
        routes::answers::DigDeeperRequest,
        routes::answers::DigDeeperResponse,
        routes::ratings::CreateRatingRequest,
        routes::ratings::RatingResponse,
    )),
    tags(
        (name = "questions", description = "Question endpoints"),
        (name = "answers", description = "Answer endpoints"),
        (name = "ratings", description = "Rating endpoints"),
    )
)]
struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt_secret: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub base_url: String,
    pub llm_chat_model: Option<String>,
    pub llm_embedding_model: Option<String>,
}

async fn health(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>, Option<String>, String)>(
        "SELECT id, display_name, email, avatar_url, role FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "id": user.0,
        "display_name": user.1,
        "email": user.2,
        "avatar_url": user.3,
        "role": user.4,
    })))
}

async fn delete_me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "UPDATE ratings SET rater_original_query = NULL, comment = NULL WHERE rater_id = $1",
    )
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("UPDATE contradiction_flags SET flagged_by = NULL WHERE flagged_by = $1")
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("UPDATE users SET display_name = 'Deleted User', email = NULL, avatar_url = NULL, provider_id = '' WHERE id = $1")
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn build_router(state: AppState) -> Router {
    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/github", get(auth::oauth::github_login))
        .route("/auth/github/callback", get(auth::oauth::github_callback))
        .route("/me", get(me).delete(delete_me))
        .route(
            "/questions",
            axum::routing::post(routes::questions::create_question)
                .get(routes::questions::list_questions),
        )
        .route(
            "/questions/search",
            get(routes::questions::search_questions),
        )
        .route(
            "/questions/preview",
            axum::routing::post(routes::questions::preview_question),
        )
        .route("/questions/{id}", get(routes::questions::get_question))
        .route("/questions/{id}/answers", get(routes::answers::get_answers))
        .route(
            "/answers/{id}",
            axum::routing::put(routes::answers::edit_answer),
        )
        .route("/answers/{id}/history", get(routes::answers::get_history))
        .route(
            "/answers/{id}/dig-deeper",
            axum::routing::post(routes::answers::dig_deeper),
        )
        .route(
            "/answers/{id}/deep-dives",
            get(routes::answers::get_deep_dives),
        )
        .route(
            "/answers/{id}/mark-stale",
            axum::routing::post(routes::answers::mark_stale),
        )
        .route(
            "/answers/{id}/ratings",
            axum::routing::post(routes::ratings::create_rating).get(routes::ratings::get_ratings),
        )
        .route(
            "/answers/{id}/ratings/redact",
            axum::routing::put(routes::ratings::redact_rating),
        )
        .route(
            "/answers/{id}/flag-contradiction",
            axum::routing::post(routes::contradictions::flag_contradiction),
        )
        .route(
            "/answers/{id}/contradictions",
            get(routes::contradictions::get_contradictions_for_answer),
        )
        .route(
            "/admin/contradictions",
            get(routes::contradictions::admin_review_queue),
        )
        .route(
            "/admin/config",
            get(routes::admin::get_config).put(routes::admin::update_config),
        )
        .route("/graph", get(routes::graph::get_graph))
        .route(
            "/graph/node/{id}",
            get(routes::graph::get_node_neighborhood),
        )
        .route("/tags", get(routes::tags::list_tags))
        .route(
            "/questions/{id}/link",
            axum::routing::post(routes::links::link_questions),
        )
        .route(
            "/questions/{id}/comments",
            axum::routing::post(routes::comments::create_question_comment)
                .get(routes::comments::get_question_comments),
        )
        .route(
            "/answers/{id}/comments",
            axum::routing::post(routes::comments::create_answer_comment)
                .get(routes::comments::get_answer_comments),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    #[cfg(debug_assertions)]
    let app =
        app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    app
}
