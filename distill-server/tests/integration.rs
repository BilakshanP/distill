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
async fn test_config_driven_answer_mode() {
    let server = setup().await;
    let (_uid, token) = create_admin_user().await;

    // Set answer_mode to community-only
    server
        .put("/admin/config")
        .authorization_bearer(&token)
        .json(&serde_json::json!({"config": {"answer_mode": "community-only"}}))
        .await;

    // Create a question — should NOT trigger AI answer
    let resp = server
        .post("/questions")
        .authorization_bearer(&token)
        .json(&serde_json::json!({"title": "No AI", "body": "test"}))
        .await;
    let q_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let resp = server.get(&format!("/questions/{}/answers", q_id)).await;
    resp.assert_status_ok();
    let answers: Vec<serde_json::Value> = resp.json();
    assert!(answers.is_empty()); // No AI answer generated
}

#[tokio::test]
async fn test_mark_stale() {
    let server = setup().await;
    let (uid, token) = create_test_user().await;
    let db = get_db().await;

    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, author_id, title, body, original_query) VALUES ($1, $2, 'Q', 'B', 'Q B')")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();
    let a_id = Uuid::new_v4();
    sqlx::query("INSERT INTO answers (id, question_id, author_type, body) VALUES ($1, $2, 'human', 'old answer')")
        .bind(a_id).bind(q_id).execute(&db).await.unwrap();

    let resp = server
        .post(&format!("/answers/{}/mark-stale", a_id))
        .authorization_bearer(&token)
        .json(&serde_json::json!({"reason": "outdated for v3"}))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["is_stale"], true);
}

#[tokio::test]
async fn test_comments_paginated() {
    let server = setup().await;
    let (uid, token) = create_test_user().await;
    let db = get_db().await;

    let q_id = Uuid::new_v4();
    sqlx::query("INSERT INTO questions (id, author_id, title, body, original_query) VALUES ($1, $2, 'Q', 'B', 'Q B')")
        .bind(q_id).bind(uid).execute(&db).await.unwrap();

    // Add 3 comments
    for i in 0..3 {
        server
            .post(&format!("/questions/{}/comments", q_id))
            .authorization_bearer(&token)
            .json(&serde_json::json!({"body": format!("comment {}", i)}))
            .await;
    }

    let resp = server
        .get(&format!("/questions/{}/comments?limit=2", q_id))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["has_more"], true);
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
        .connect("postgres://distill_app:distill@localhost:5432/distill")
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
