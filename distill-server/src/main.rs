use distill_server::{build_router, AppState};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = distill_server::config::Config::from_env();

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
        llm_chat_model: cfg.llm_chat_model.clone(),
        llm_embedding_model: cfg.llm_embedding_model.clone(),
    };

    if cfg.llm_chat_model.is_none() || cfg.llm_embedding_model.is_none() {
        tracing::warn!("⚠️  LLM models not fully configured — AI features disabled (set LLM_CHAT_MODEL and LLM_EMBEDDING_MODEL)");
    }

    let app = build_router(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("listening on {}", addr);

    // Rate limiting: 60 requests/min per IP
    let governor_conf = tower_governor::governor::GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(60)
        .finish()
        .unwrap();
    let app = app.layer(tower_governor::GovernorLayer::new(governor_conf));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
