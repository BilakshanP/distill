pub mod auth;
pub mod config;
pub mod config_enums;
pub mod error;
pub mod jobs;
pub mod routes;
pub mod services;

/// Bump this when embedding pipeline changes (model, chunking, normalization).
/// Re-embed endpoint targets questions with version < this value.
pub const EMBEDDING_VERSION: i32 = 1;

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use sqlx::PgPool;
use std::collections::HashSet;
use tower_http::trace::TraceLayer;
#[cfg(debug_assertions)]
use utoipa::OpenApi;
#[cfg(debug_assertions)]
use utoipa_swagger_ui::SwaggerUi;

use auth::middleware::AuthUser;

#[cfg(debug_assertions)]
struct SecurityAddon;

#[cfg(debug_assertions)]
impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            utoipa::openapi::security::SecurityScheme::Http(utoipa::openapi::security::Http::new(
                utoipa::openapi::security::HttpAuthScheme::Bearer,
            )),
        );
    }
}

#[cfg(debug_assertions)]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Distill API",
        version = "0.4.0",
        license(name = "MIT OR Apache-2.0", url = "https://opensource.org/licenses/MIT")
    ),
    security(
        ("bearer" = [])
    ),
    modifiers(&SecurityAddon),
    paths(
        routes::questions::create_question,
        routes::questions::list_questions,
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
        routes::contradictions::flag_contradiction,
        routes::contradictions::get_contradictions_for_answer,
        routes::contradictions::admin_review_queue,
        routes::graph::get_graph,
        routes::graph::get_node_neighborhood,
        routes::tags::list_tags,
        routes::comments::create_question_comment,
        routes::comments::get_question_comments,
        routes::comments::create_answer_comment,
        routes::comments::get_answer_comments,
        routes::links::link_questions,
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
        routes::contradictions::FlagContradictionRequest,
        routes::contradictions::ContradictionResponse,
        routes::graph::GraphResponse,
        routes::graph::GraphNode,
        routes::graph::GraphEdge,
        routes::tags::TagCount,
        routes::comments::CreateCommentRequest,
        routes::comments::CommentResponse,
        routes::links::LinkRequest,
        routes::links::LinkResponse,
    )),
    tags(
        (name = "questions", description = "Question endpoints"),
        (name = "answers", description = "Answer endpoints"),
        (name = "ratings", description = "Rating endpoints"),
        (name = "contradictions", description = "Contradiction detection"),
        (name = "graph", description = "Knowledge graph"),
        (name = "tags", description = "Tag endpoints"),
        (name = "comments", description = "Comments on questions and answers"),
        (name = "links", description = "Manual question linking"),
    )
)]
struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt_secret: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub base_url: String,
    pub llm_chat_model: Option<String>,
    pub llm_embedding_model: Option<String>,
    pub admin_emails: HashSet<String>,
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

    sqlx::query("UPDATE answer_ratings SET comment = NULL WHERE rater_id = $1")
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .ok();

    sqlx::query(
        "UPDATE discussions SET body = '[deleted]', is_deleted = true WHERE author_id = $1",
    )
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await
    .ok();

    let anon_name = format!("[deleted-{}]", &auth.user_id.to_string()[..8]);
    sqlx::query("UPDATE users SET display_name = $2, email = NULL, avatar_url = NULL, provider_id = $3 WHERE id = $1")
        .bind(auth.user_id)
        .bind(&anon_name)
        .bind(format!("deleted-{}", auth.user_id))
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
        .route("/auth/github/web", get(auth::oauth::github_login_web))
        .route("/auth/github/callback", get(auth::oauth::github_callback))
        .route("/auth/google", get(auth::oauth::google_login))
        .route("/auth/google/callback", get(auth::oauth::google_callback))
        .route(
            "/auth/token",
            axum::routing::post(auth::oauth::exchange_token),
        )
        .route("/auth/config", get(auth::oauth::auth_config))
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
        .route(
            "/questions/{id}/answers",
            get(routes::answers::get_answers).post(routes::answers::create_answer),
        )
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
            "/answers/{id}/ratings/mine",
            axum::routing::delete(routes::ratings::delete_rating),
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
        .route(
            "/admin/user-quota",
            axum::routing::put(routes::admin::set_user_quota),
        )
        .route(
            "/admin/re-embed",
            axum::routing::post(routes::admin::re_embed),
        )
        .route(
            "/admin/users/{id}/promote",
            axum::routing::put(routes::admin::promote_user),
        )
        .route(
            "/admin/tenants",
            axum::routing::post(routes::tenants::create_tenant).get(routes::tenants::list_tenants),
        )
        .route(
            "/admin/tenants/assign",
            axum::routing::put(routes::tenants::assign_tenant),
        )
        .route("/admin/jobs", get(routes::admin::list_jobs))
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
        // Wiki answers
        .route(
            "/questions/{id}/wiki-answer",
            get(routes::wiki_answers::get_wiki_answer).put(routes::wiki_answers::edit_wiki_answer),
        )
        .route(
            "/questions/{id}/wiki-answer/history",
            get(routes::wiki_answers::get_wiki_answer_history),
        )
        .route("/revisions/{id}", get(routes::wiki_answers::get_revision))
        // Discussions
        .route(
            "/questions/{id}/discussions",
            axum::routing::post(routes::discussions::create_discussion)
                .get(routes::discussions::list_discussions),
        )
        // Discussion votes
        .route(
            "/discussions/{id}/vote",
            axum::routing::post(routes::discussion_votes::vote_discussion),
        )
        .layer(TraceLayer::new_for_http());

    let app = if std::env::var("DEBUG_REQUESTS").unwrap_or_default() == "true" {
        app.layer(axum::middleware::from_fn(debug_log_middleware))
    } else {
        app
    };

    let app = app.with_state(state);

    #[cfg(debug_assertions)]
    let app =
        app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    app
}

async fn debug_log_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();
    let resp = next.run(req).await;
    tracing::info!(
        "{} {} → {} ({:?})",
        method,
        uri,
        resp.status(),
        start.elapsed()
    );
    resp
}
