use distill_sdk::Client;
use distill_server::{auth::jwt, build_router, AppState};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("../distill-server/migrations");

async fn start_server() -> (String, Uuid) {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://distill:distill@localhost:5432/distill".into());

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    MIGRATOR.run(&db).await.expect("Failed to run migrations");

    // Create a test user
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, provider, provider_id, display_name, role) VALUES ($1, 'test', $2, 'SDK Test User', 'admin')")
        .bind(user_id)
        .bind(user_id.to_string())
        .execute(&db)
        .await
        .unwrap();

    let state = AppState {
        db,
        jwt_secret: "sdk-test-secret".into(),
        github_client_id: "test".into(),
        github_client_secret: "test".into(),
        base_url: "http://localhost:0".into(),
        llm_chat_model: None,
        llm_embedding_model: None,
    };

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let base_url = format!("http://{}", addr);
    (base_url, user_id)
}

#[tokio::test]
async fn test_sdk_e2e_flow() {
    let (base_url, user_id) = start_server().await;
    let token = jwt::create_token(user_id, "sdk-test-secret").unwrap();

    let client = Client::new(&base_url).with_token(&token);

    // Health
    let health = client.health().await.unwrap();
    assert_eq!(health.status, "ok");

    // Create question
    let q = client
        .create_question("SDK test question", "Does the SDK work?", &["sdk", "test"])
        .await
        .unwrap();
    assert_eq!(q.title, "SDK test question");
    assert_eq!(q.tags, vec!["sdk", "test"]);

    // Get question
    let fetched = client.get_question(q.id).await.unwrap();
    assert_eq!(fetched.id, q.id);

    // Search
    let results = client.search("SDK test", None).await.unwrap();
    assert!(!results.is_empty());

    // Search with tag filter
    let results = client.search("SDK", Some("sdk")).await.unwrap();
    assert!(!results.is_empty());

    // Tags
    let tags = client.list_tags(Some("sdk"), None).await.unwrap();
    assert!(tags.iter().any(|t| t.tag == "sdk"));

    // Comment on question
    let comment = client
        .create_question_comment(q.id, "This is a test comment")
        .await
        .unwrap();
    assert_eq!(comment.body, "This is a test comment");

    let comments = client.get_question_comments(q.id).await.unwrap();
    assert_eq!(comments.len(), 1);

    // Get answers (empty since no LLM configured)
    let answers = client.get_answers(q.id).await.unwrap();
    assert!(answers.is_empty());

    // Preview
    let preview = client.preview("SDK question", "test").await.unwrap();
    assert!(preview.rephrased.is_none()); // no LLM

    // Graph
    let graph = client.get_graph().await.unwrap();
    assert!(!graph.nodes.is_empty());

    // Admin config
    let config = client.get_config().await.unwrap();
    assert!(config.config.contains_key("answer_mode"));

    // Link questions (need a second question)
    let q2 = client
        .create_question("Another question", "Related to first", &["sdk"])
        .await
        .unwrap();
    let link = client.link_questions(q.id, q2.id, "related").await.unwrap();
    assert_eq!(link.link_type, "related");
}
