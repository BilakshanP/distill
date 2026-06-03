# Contributing to Distill

## Prerequisites

- Rust 1.93+ (`rustup update`)
- Docker (for PostgreSQL + pgvector)
- A GitHub OAuth App (for auth testing)

## Setup

```bash
git clone https://github.com/BilakshanP/distill.git && cd distill

# Set up git hooks (required)
git config core.hooksPath .githooks

# Start the database
docker compose up -d

# Configure environment
cp .env.example .env
# Edit .env with your credentials

# Run the server
cargo run -p distill-server
```

## Git Hooks

This project uses `.githooks/` for git hooks. The `pre-commit` hook runs `cargo fmt --check` and rejects unformatted code.

After cloning, run:
```bash
git config core.hooksPath .githooks
```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` to check for common issues
- All code must compile with zero warnings

## Running Tests

```bash
# Requires a running PostgreSQL instance
cargo test -p distill-server
```

Tests do not depend on GitHub OAuth or any LLM API key — they use direct DB inserts and local JWT generation.

## Project Structure

- `distill-server/` — API server (Axum + sqlx + genai)
- `distill-sdk/` — Typed Rust client SDK
- `.githooks/` — Git hooks (pre-commit: cargo fmt check)

## Swagger / OpenAPI

Swagger UI is available at `/swagger-ui` in debug builds.

### Getting a bearer token for Swagger testing

1. Start the server: `cargo run -p distill-server`
2. Visit `http://localhost:3000/auth/github` in your browser
3. Complete the GitHub OAuth flow
4. You'll be redirected with a JWT token in the response
5. In Swagger UI, click the **Authorize** 🔒 button at the top
6. Enter: `Bearer <your-token>` (or just the token — Swagger adds the prefix)
7. All locked endpoints will now send the token automatically

For quick local testing without OAuth, you can generate a token directly:

```bash
# In a Rust test or script:
distill_server::auth::jwt::create_token(user_id, "your-jwt-secret")
```

### Annotation requirements

### Annotation requirements

Every public endpoint **must** have a full `#[utoipa::path]` annotation including:

- HTTP method and path
- `request_body = Type` (if applicable)
- `responses((status = CODE, body = Type))` with response schemas
- `tag = "section"`

The response/request types must derive `utoipa::ToSchema`.

New endpoints must also be registered in:
1. `paths(...)` in `lib.rs` `#[openapi(...)]`
2. `components(schemas(...))` if introducing new types
3. `tags(...)` if introducing a new tag group

Swagger UI is available at `/swagger-ui` in debug builds. Verify your annotations render correctly before submitting.
