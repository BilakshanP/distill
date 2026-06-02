use axum_test::TestServer;
use distill_server::{auth::jwt, build_router, AppState};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

async fn setup() -> TestServer {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://distill:distill@localhost:5432/distill".into());

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to test DB");

    MIGRATOR.run(&db).await.expect("Failed to run migrations");

    let state = AppState {
        db,
        jwt_secret: "test-secret".into(),
        github_client_id: "test".into(),
        github_client_secret: "test".into(),
        base_url: "http://localhost:3000".into(),
        llm_chat_model: None,
        llm_embedding_model: None,
    };

    let app = build_router(state);
    TestServer::new(app)
}

async fn get_db() -> sqlx::PgPool {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://distill:distill@localhost:5432/distill".into());
    PgPoolOptions::new().connect(&db_url).await.unwrap()
}

async fn create_test_user() -> (Uuid, String) {
    let db = get_db().await;
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, provider, provider_id, display_name, role) VALUES ($1, 'test', $2, 'Test User', 'user')"
    )
    .bind(user_id)
    .bind(user_id.to_string())
    .execute(&db)
    .await
    .unwrap();

    let token = jwt::create_token(user_id, "test-secret").unwrap();
    (user_id, token)
}

async fn create_admin_user() -> (Uuid, String) {
    let db = get_db().await;
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, provider, provider_id, display_name, role) VALUES ($1, 'test', $2, 'Admin User', 'admin')"
    )
    .bind(user_id)
    .bind(user_id.to_string())
    .execute(&db)
    .await
    .unwrap();

    let token = jwt::create_token(user_id, "test-secret").unwrap();
    (user_id, token)
}

#[tokio::test]
async fn test_health() {
    let server = setup().await;
    let resp = server.get("/health").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn test_me_unauthorized() {
    let server = setup().await;
    let resp = server.get("/me").await;
    resp.assert_status_unauthorized();
}

#[tokio::test]
async fn test_me_authorized() {
    let server = setup().await;
    let (_uid, token) = create_test_user().await;
    let resp = server.get("/me").authorization_bearer(&token).await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn test_create_and_get_question() {
    let server = setup().await;
    let (_uid, token) = create_test_user().await;

    let resp = server
        .post("/questions")
        .authorization_bearer(&token)
        .json(&serde_json::json!({"title": "Test Q", "body": "Test body", "tags": ["t"]}))
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);

    let body: serde_json::Value = resp.json();
    let id = body["id"].as_str().unwrap();

    let resp = server.get(&format!("/questions/{}", id)).await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn test_search() {
    let server = setup().await;
    let (_uid, token) = create_test_user().await;

    server.post("/questions")
        .authorization_bearer(&token)
        .json(&serde_json::json!({"title": "Rust lifetimes explained", "body": "How do lifetimes work"}))
        .await;

    let resp = server.get("/questions/search?q=lifetimes").await;
    resp.assert_status_ok();
    let results: Vec<serde_json::Value> = resp.json();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_answers_and_edit() {
    let server = setup().await;
    let (uid, token) = create_test_user().await;
    let db = get_db().await;

    // Create question + answer
    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, author_id, title, body, original_query) VALUES ($1, $2, 'Q', 'B', 'Q B')")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();

    let a_id = Uuid::new_v4();
    sqlx::query("INSERT INTO answers (id, question_id, author_type, body) VALUES ($1, $2, 'human', 'Original')")
        .bind(a_id).bind(q_id).execute(&db).await.unwrap();

    // Get answers
    let resp = server.get(&format!("/questions/{}/answers", q_id)).await;
    resp.assert_status_ok();
    let answers: Vec<serde_json::Value> = resp.json();
    assert_eq!(answers.len(), 1);

    // Edit
    let resp = server
        .put(&format!("/answers/{}", a_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"body": "Edited", "edit_message": "fix"}))
        .await;
    resp.assert_status_ok();

    // History
    let resp = server.get(&format!("/answers/{}/history", a_id)).await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let history = body["data"].as_array().unwrap();
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn test_ratings() {
    let server = setup().await;
    let (uid, token) = create_test_user().await;
    let db = get_db().await;

    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, author_id, title, body, original_query) VALUES ($1, $2, 'Q', 'B', 'Q B')")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();

    let a_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO answers (id, question_id, author_type, body) VALUES ($1, $2, 'ai', 'ans')",
    )
    .bind(a_id)
    .bind(q_id)
    .execute(&db)
    .await
    .unwrap();

    let resp = server
        .post(&format!("/answers/{}/ratings", a_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"score": 5, "comment": "Great", "rater_original_query": "test"}))
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);

    let resp = server.get(&format!("/answers/{}/ratings", a_id)).await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let ratings = body["data"].as_array().unwrap();
    assert_eq!(ratings[0]["score"], 5);
}

#[tokio::test]
async fn test_contradiction_flag() {
    let server = setup().await;
    let (uid, token) = create_test_user().await;
    let db = get_db().await;

    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, author_id, title, body, original_query) VALUES ($1, $2, 'Q', 'B', 'Q B')")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();

    let a1 = Uuid::new_v4();
    let a2 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO answers (id, question_id, author_type, body) VALUES ($1, $2, 'human', 'yes')",
    )
    .bind(a1)
    .bind(q_id)
    .execute(&db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO answers (id, question_id, author_type, body) VALUES ($1, $2, 'human', 'no')",
    )
    .bind(a2)
    .bind(q_id)
    .execute(&db)
    .await
    .unwrap();

    let resp = server
        .post(&format!("/answers/{}/flag-contradiction", a1))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"contradicts_answer_id": a2, "explanation": "opposite"}))
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);

    // Admin queue requires admin role
    let (_admin_uid, admin_token) = create_admin_user().await;
    let resp = server
        .get("/admin/contradictions")
        .authorization_bearer(&admin_token)
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["explanation"] == "opposite"));
}

#[tokio::test]
async fn test_graph() {
    let server = setup().await;
    let resp = server.get("/graph").await;
    resp.assert_status_ok();
    let graph: serde_json::Value = resp.json();
    assert!(graph["nodes"].is_array());
}

#[tokio::test]
async fn test_admin_config() {
    let server = setup().await;
    let (_uid, token) = create_admin_user().await;

    let resp = server
        .get("/admin/config")
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();

    let resp = server
        .put("/admin/config")
        .authorization_bearer(&token)
        .json(&serde_json::json!({"config": {"rating_scale": "thumbs"}}))
        .await;
    resp.assert_status_ok();
}
