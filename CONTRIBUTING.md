# Contributing to Distill

## Prerequisites

- Rust 1.93+ (`rustup update`)
- Docker (for PostgreSQL + pgvector)
- A GitHub OAuth App (for auth testing)

## Setup

```bash
git clone <repo-url> && cd distill

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
