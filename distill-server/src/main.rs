mod auth;
mod config;
mod routes;

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use sqlx::postgres::PgPoolOptions;
use sqlx::{migrate::Migrator, PgPool};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use auth::middleware::AuthUser;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt_secret: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub base_url: String,
    pub llm_api_key: Option<String>,
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();
    let cfg = config::Config::from_env();

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await
        .expect("Failed to connect to database");

    if cfg.auto_migrate {
        tracing::info!("running migrations");
        MIGRATOR.run(&db).await.expect("Failed to run migrations");
    }

    let state = AppState {
        db,
        jwt_secret: cfg.jwt_secret,
        github_client_id: cfg.github_client_id,
        github_client_secret: cfg.github_client_secret,
        base_url: cfg.base_url,
        llm_api_key: cfg.llm_api_key,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/github", get(auth::oauth::github_login))
        .route("/auth/github/callback", get(auth::oauth::github_callback))
        .route("/me", get(me))
        .route("/questions", axum::routing::post(routes::questions::create_question))
        .route("/questions/{id}", get(routes::questions::get_question))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
