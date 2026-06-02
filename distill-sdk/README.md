# distill-sdk

Typed Rust client for the Distill API.

## Usage

```toml
[dependencies]
distill-sdk = { path = "../distill-sdk" }
tokio = { version = "1", features = ["full"] }
```

```rust
use distill_sdk::Client;

#[tokio::main]
async fn main() {
    let client = Client::new("http://localhost:3000")
        .with_token("your-jwt-token");

    // Search for questions
    let results = client.search("pgvector setup", None).await.unwrap();
    for r in &results {
        println!("{} (score: {:.3})", r.title, r.score);
    }

    // Create a question
    let q = client
        .create_question("How to configure pgvector?", "Need help with HNSW", &["postgres"])
        .await
        .unwrap();

    // Get AI-generated answers
    let answers = client.get_answers(q.id).await.unwrap();

    // Rate an answer
    if let Some(a) = answers.first() {
        client.rate_answer(a.id, 5, Some("Helpful!"), Some("pgvector setup")).await.unwrap();
    }

    // Dig deeper
    if let Some(a) = answers.first() {
        let deep = client.dig_deeper(a.id, "What about IVFFlat vs HNSW?", true).await.unwrap();
        println!("{}", deep.response);
    }
}
```

## Available Methods

### Questions
- `create_question(title, body, tags)` → `QuestionResponse`
- `get_question(id)` → `QuestionResponse`
- `search(query, tags)` → `Vec<SearchResult>`
- `preview(title, body)` → `PreviewResponse`
- `link_questions(question_id, target_id, link_type)` → `LinkResponse`

### Answers
- `get_answers(question_id)` → `Vec<AnswerResponse>`
- `edit_answer(answer_id, body, message)` → `AnswerResponse`
- `get_history(answer_id)` → `Vec<EditHistoryEntry>`
- `mark_stale(answer_id, reason)` → `AnswerResponse`
- `dig_deeper(answer_id, prompt, include_comments)` → `DigDeeperResponse`
- `get_deep_dives(answer_id)` → `Vec<DigDeeperResponse>`

### Ratings
- `rate_answer(answer_id, score, comment, query)` → `RatingResponse`
- `get_ratings(answer_id, after)` → `Paginated<RatingResponse>`
- `redact_rating(answer_id)` → `()`

### Comments
- `create_question_comment(question_id, body)` → `CommentResponse`
- `get_question_comments(question_id)` → `Vec<CommentResponse>`
- `create_answer_comment(answer_id, body)` → `CommentResponse`
- `get_answer_comments(answer_id)` → `Vec<CommentResponse>`

### Contradictions
- `flag_contradiction(answer_id, contradicts_id, explanation)` → `ContradictionResponse`
- `get_contradictions(answer_id)` → `Vec<ContradictionResponse>`

### Tags
- `list_tags(query, limit)` → `Vec<TagCount>`

### Graph
- `get_graph()` → `GraphResponse`
- `get_node(id)` → `GraphResponse`

### Admin
- `get_config()` → `ConfigResponse`
- `update_config(config)` → `ConfigResponse`
- `get_admin_contradictions(after)` → `Paginated<ContradictionResponse>`

### Account
- `delete_account()` → `()`
