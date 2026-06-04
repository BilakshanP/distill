use axum_test::TestServer;
use distill_server::{auth::jwt, build_router, AppState};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use tokio::sync::OnceCell;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
static TEST_DB: OnceCell<sqlx::PgPool> = OnceCell::const_new();

fn test_db_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://distill:distill@localhost:5432/distill_test".into())
}

async fn init_test_db() -> sqlx::PgPool {
    dotenvy::dotenv().ok();
    // Create distill_test if not exists
    let admin_url = "postgres://distill:distill@localhost:5432/postgres";
    if let Ok(conn) = PgPoolOptions::new()
        .max_connections(1)
        .connect(admin_url)
        .await
    {
        sqlx::query("CREATE DATABASE distill_test OWNER distill")
            .execute(&conn)
            .await
            .ok();
    }

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&test_db_url())
        .await
        .expect("Failed to connect to test DB");
    MIGRATOR.run(&pool).await.expect("Failed to run migrations");
    pool
}

async fn get_db() -> sqlx::PgPool {
    TEST_DB.get_or_init(init_test_db).await.clone()
}

async fn setup() -> TestServer {
    let db = get_db().await;

    let state = AppState {
        db,
        jwt_secret: "test-secret".into(),
        github_client_id: "test".into(),
        github_client_secret: "test".into(),
        google_client_id: None,
        google_client_secret: None,
        base_url: "http://localhost:3000".into(),
        llm_chat_model: None,
        llm_embedding_model: None,
        admin_emails: HashSet::new(),
    };

    let app = build_router(state);
    TestServer::new(app)
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
async fn test_wiki_answer_and_history() {
    let server = setup().await;
    let (uid, token) = create_test_user().await;
    let db = get_db().await;

    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, author_id, title, body, original_query) VALUES ($1, $2, 'Q', 'B', 'Q B')")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();

    // No answer yet
    let resp = server
        .get(&format!("/questions/{}/wiki-answer", q_id))
        .await;
    resp.assert_status(axum::http::StatusCode::NOT_FOUND);

    // Create/edit wiki answer
    let resp = server
        .put(&format!("/questions/{}/wiki-answer", q_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"body": "First version", "edit_message": "initial"}))
        .await;
    resp.assert_status_ok();

    // Edit again
    let resp = server
        .put(&format!("/questions/{}/wiki-answer", q_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"body": "Second version", "edit_message": "updated"}))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["body"], "Second version");

    // History
    let resp = server
        .get(&format!("/questions/{}/wiki-answer/history", q_id))
        .await;
    resp.assert_status_ok();
    let history: Vec<serde_json::Value> = resp.json();
    assert_eq!(history.len(), 2);
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

#[tokio::test]
async fn test_list_questions_paginated() {
    let server = setup().await;
    let (_uid, token) = create_test_user().await;

    // Create 3 questions
    for i in 0..3 {
        server
            .post("/questions")
            .authorization_bearer(&token)
            .json(&serde_json::json!({"title": format!("Q{}", i), "body": "body"}))
            .await;
    }

    // List with limit=2
    let resp = server.get("/questions?limit=2").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["has_more"], true);
    assert!(body["next_cursor"].is_string());

    // Fetch next page — should have remaining items
    let cursor = body["next_cursor"].as_str().unwrap();
    let resp = server
        .get(&format!("/questions?limit=2&after={}", cursor))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(!body["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_rls_tenant_isolation() {
    let db = get_db().await;

    // RLS doesn't apply to superusers — create app role for testing
    sqlx::query("DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'distill_app') THEN CREATE ROLE distill_app LOGIN PASSWORD 'distill'; END IF; END $$")
        .execute(&db).await.unwrap();
    sqlx::query("GRANT ALL ON ALL TABLES IN SCHEMA public TO distill_app")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA public TO distill_app")
        .execute(&db)
        .await
        .unwrap();

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let user_a = Uuid::new_v4();

    // Insert as superuser (bypasses RLS)
    sqlx::query("INSERT INTO users (id, tenant_id, provider, provider_id, display_name) VALUES ($1, $2, 'test', $3, 'A')")
        .bind(user_a).bind(tenant_a).bind(user_a.to_string()).execute(&db).await.unwrap();

    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, tenant_id, author_id, title, body, original_query) VALUES ($1, $2, $3, 'Secret', 'Body', 'Secret Body')")
        .bind(q_id).bind(tenant_a).bind(user_a).execute(&db).await.unwrap();

    // Connect as app user (RLS applies)
    let app_db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect("postgres://distill_app:distill@localhost:5432/distill_test")
        .await
        .unwrap();

    // As tenant B — should NOT see tenant A's question
    let mut conn = app_db.acquire().await.unwrap();
    sqlx::query("SELECT set_config('app.current_tenant', $1::text, false)")
        .bind(tenant_b.to_string())
        .execute(&mut *conn)
        .await
        .unwrap();
    let result: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM questions WHERE id = $1")
        .bind(q_id)
        .fetch_optional(&mut *conn)
        .await
        .unwrap();
    assert!(
        result.is_none(),
        "tenant B should not see tenant A's question"
    );

    // As tenant A — should see it
    sqlx::query("SELECT set_config('app.current_tenant', $1::text, false)")
        .bind(tenant_a.to_string())
        .execute(&mut *conn)
        .await
        .unwrap();
    let result: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM questions WHERE id = $1")
        .bind(q_id)
        .fetch_optional(&mut *conn)
        .await
        .unwrap();
    assert!(result.is_some(), "tenant A should see own question");
}

#[tokio::test]
async fn test_search_hybrid_vs_keyword() {
    let server = setup().await;
    let (_uid, token) = create_test_user().await;

    // Insert questions with distinct keywords
    server.post("/questions").authorization_bearer(&token)
        .json(&serde_json::json!({"title": "PostgreSQL indexing strategies", "body": "BTREE GIN GiST BRIN"}))
        .await;
    server
        .post("/questions")
        .authorization_bearer(&token)
        .json(&serde_json::json!({"title": "Redis caching patterns", "body": "TTL eviction LRU"}))
        .await;

    // Search should find postgres question by keyword
    let resp = server.get("/questions/search?q=postgresql+indexing").await;
    resp.assert_status_ok();
    let results: Vec<serde_json::Value> = resp.json();
    assert!(!results.is_empty());
    assert!(results[0]["title"].as_str().unwrap().contains("PostgreSQL"));

    // Should NOT find redis when searching for postgres
    assert!(!results
        .iter()
        .any(|r| r["title"].as_str().unwrap().contains("Redis")));
}

#[tokio::test]
async fn test_search_tag_filter() {
    let server = setup().await;
    let (_uid, token) = create_test_user().await;

    server.post("/questions").authorization_bearer(&token)
        .json(&serde_json::json!({"title": "Axum middleware", "body": "tower layers", "tags": ["rust", "web"]}))
        .await;
    server.post("/questions").authorization_bearer(&token)
        .json(&serde_json::json!({"title": "Axum extractors", "body": "custom extractors", "tags": ["rust", "api"]}))
        .await;

    // Filter by tag=api should only return extractors
    let resp = server.get("/questions/search?q=axum&tags=api").await;
    resp.assert_status_ok();
    let results: Vec<serde_json::Value> = resp.json();
    assert!(results.iter().all(|r| r["tags"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("api"))));
}

#[tokio::test]
#[ignore] // slow: tests exponential backoff (~90s)
async fn test_job_queue_lifecycle() {
    let db = get_db().await;

    // Enqueue a job
    let job_id = distill_server::jobs::enqueue(
        &db,
        &distill_server::jobs::JobPayload::GenerateEmbedding {
            question_id: Uuid::new_v4(),
            text: "test".to_string(),
            model: "nonexistent-model".to_string(),
        },
    )
    .await
    .unwrap();

    // Verify it's pending
    let status: String = sqlx::query_scalar("SELECT status FROM jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(status, "pending");

    // Process — will fail (no real LLM)
    distill_server::jobs::process_pending(&db).await;

    // Should be back to pending with backoff (attempt 1 < max_attempts 3)
    let (status, attempts): (String, i32) =
        sqlx::query_as("SELECT status, attempts FROM jobs WHERE id = $1")
            .bind(job_id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(status, "pending");
    assert_eq!(attempts, 1);

    // Process 2 more times to exhaust retries
    // Set next_attempt_at to now to bypass backoff for test
    sqlx::query("UPDATE jobs SET next_attempt_at = now() WHERE id = $1")
        .bind(job_id)
        .execute(&db)
        .await
        .unwrap();
    distill_server::jobs::process_pending(&db).await;
    sqlx::query("UPDATE jobs SET next_attempt_at = now() WHERE id = $1")
        .bind(job_id)
        .execute(&db)
        .await
        .unwrap();
    distill_server::jobs::process_pending(&db).await;

    let status: String = sqlx::query_scalar("SELECT status FROM jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}

#[tokio::test]
async fn test_admin_promote_user() {
    let server = setup().await;
    let (user_id, _user_token) = create_test_user().await;
    let (_admin_id, admin_token) = create_admin_user().await;

    // Non-admin cannot access admin endpoints
    let resp = server
        .get("/admin/config")
        .authorization_bearer(&_user_token)
        .await;
    resp.assert_status(axum::http::StatusCode::FORBIDDEN);

    // Admin promotes user
    let resp = server
        .put(&format!("/admin/users/{}/promote", user_id))
        .authorization_bearer(&admin_token)
        .await;
    resp.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Promoted user can now access admin endpoints
    let resp = server
        .get("/admin/config")
        .authorization_bearer(&_user_token)
        .await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn test_non_admin_cannot_promote() {
    let server = setup().await;
    let (target_id, _) = create_test_user().await;
    let (_, user_token) = create_test_user().await;

    let resp = server
        .put(&format!("/admin/users/{}/promote", target_id))
        .authorization_bearer(&user_token)
        .await;
    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_discussions_and_votes() {
    let server = setup().await;
    let (uid, token) = create_test_user().await;
    let db = get_db().await;

    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, author_id, title, body, original_query) VALUES ($1, $2, 'Q', 'B', 'Q B')")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();

    // Create top-level discussion
    let resp = server
        .post(&format!("/questions/{}/discussions", q_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"body": "Top level comment"}))
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);
    let d: serde_json::Value = resp.json();
    let d_id = d["id"].as_str().unwrap();
    assert_eq!(d["depth"], 0);

    // Reply
    let resp = server
        .post(&format!("/questions/{}/discussions", q_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"body": "Reply", "parent_id": d_id}))
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);
    let reply: serde_json::Value = resp.json();
    assert_eq!(reply["depth"], 1);

    // List
    let resp = server
        .get(&format!("/questions/{}/discussions", q_id))
        .await;
    resp.assert_status_ok();
    let list: Vec<serde_json::Value> = resp.json();
    assert_eq!(list.len(), 2);

    // Vote up
    let resp = server
        .post(&format!("/discussions/{}/vote", d_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"direction": 1}))
        .await;
    resp.assert_status_ok();
    let v: serde_json::Value = resp.json();
    assert_eq!(v["score"], 1);
    assert_eq!(v["user_vote"], 1);

    // Vote same direction = remove
    let resp = server
        .post(&format!("/discussions/{}/vote", d_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"direction": 1}))
        .await;
    resp.assert_status_ok();
    let v: serde_json::Value = resp.json();
    assert_eq!(v["score"], 0);
    assert!(v["user_vote"].is_null());
}

#[tokio::test]
async fn test_delete_account() {
    let server = setup().await;
    let (uid, token) = create_test_user().await;
    let db = get_db().await;

    // Create question + discussion
    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, author_id, title, body, original_query) VALUES ($1, $2, 'Q', 'B', 'Q B')")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();
    sqlx::query("INSERT INTO discussions (question_id, author_id, body, depth) VALUES ($1, $2, 'comment', 0)")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();

    // Delete account
    let resp = server.delete("/me").authorization_bearer(&token).await;
    resp.assert_status(axum::http::StatusCode::NO_CONTENT);

    // User should be anonymized
    let user: (String,) = sqlx::query_as("SELECT display_name FROM users WHERE id = $1")
        .bind(uid)
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(user.0.starts_with("[deleted-"));
}
