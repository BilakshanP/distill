# Distill

A smart Q&A platform with intelligent question deduplication, transparent ratings, AI-generated answers, and LLM-powered contradiction detection.

## What it does

When someone asks a question, Distill:

1. Finds existing matches using hybrid search (semantic + keyword)
2. Suggests better phrasing for the query
3. Surfaces related questions
4. Stores the new query to improve future matching
5. Generates an AI answer from existing KB context
6. Lets users rate answers with full transparency (rater's context visible publicly)
7. Detects contradictions between answers and flags them for review
8. Allows users to "dig deeper" on any answer via LLM
9. Visualizes knowledge relationships as a graph (topics, similarity, contradictions)

## Quick Start

### Prerequisites

- Rust 1.93+
- Docker (for PostgreSQL + pgvector)
- A Gemini/OpenAI/Anthropic API key (optional for dev, required for AI features)

### Setup

```bash
git clone https://github.com/BilakshanP/distill.git && cd distill

# Start the database
docker compose up -d

# Configure environment
cp .env.example .env
# Edit .env with your GitHub OAuth credentials and LLM API key

# Run the server
cargo run -p distill-server
```

Server starts at `http://localhost:3000`.

### Authentication

1. Create a GitHub OAuth App at https://github.com/settings/developers
   - Callback URL: `http://localhost:3000/auth/github/callback`
2. Add credentials to `.env`
3. Visit `http://localhost:3000/auth/github` to login and get a JWT token

### Running Tests

```bash
# Requires a running PostgreSQL instance
cargo test -p distill-server
```

### Migrations

Migrations run automatically on boot if `AUTO_MIGRATE=true` (default in `.env.example`).

To run manually:
```bash
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run --source distill-server/migrations
```

To reset the database (destroys all data):
```bash
docker compose down -v && docker compose up -d
```

### Swagger UI

```bash
cargo run -p distill-server
```

Visit `http://localhost:3000/swagger-ui` for interactive API documentation.

Swagger UI is automatically available in debug builds and stripped from release builds.

```bash
cargo run -p distill-server --release  # No swagger, smaller binary
```

## Documentation

- **[API Reference](docs/API.md)** — Full endpoint list, pagination, config keys
- **[SDK Usage](distill-sdk/README.md)** — Typed Rust client library
- **[Contributing](CONTRIBUTING.md)** — Dev setup, git hooks, code style
- **[Design Document](docs/PLAN.md)** — Original architecture and implementation plan

## Tech Stack

- **Language:** Rust
- **Web framework:** Axum
- **Database:** PostgreSQL + pgvector
- **Search:** Hybrid BM25 (tsvector) + vector similarity, merged via RRF
- **LLM:** genai (multi-provider: Gemini, OpenAI, Anthropic, Ollama, etc.)
- **Diff engine:** diffy-imara
- **Auth:** GitHub OAuth + JWT

## Project Structure

```
distill/
├── distill-server/        # API server
│   ├── src/
│   │   ├── main.rs        # Entry point
│   │   ├── lib.rs         # Router + AppState (shared with tests)
│   │   ├── config.rs      # Environment config
│   │   ├── auth/          # OAuth + JWT + middleware
│   │   └── routes/        # All endpoint handlers
│   ├── migrations/        # SQL migrations (auto-run on boot)
│   └── tests/             # Integration tests
├── distill-sdk/           # Typed Rust client SDK
├── docker-compose.yml     # PostgreSQL + pgvector
└── PLAN.md                # Full implementation plan
```

## Configuration

All config is via environment variables (see `.env.example`):

- `DATABASE_URL` — PostgreSQL connection string
- `AUTO_MIGRATE` — Run migrations on boot (true/false)
- `JWT_SECRET` — Secret for JWT signing
- `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET` — OAuth credentials
- `LLM_CHAT_MODEL` — Model for chat/rephrase/contradiction (e.g., `gemini-2.5-flash`)
- `LLM_EMBEDDING_MODEL` — Model for embeddings (e.g., `gemini-embedding-001`)
- Provider API key (`GEMINI_API_KEY`, `OPENAI_API_KEY`, etc.)

## License

MIT OR Apache-2.0
