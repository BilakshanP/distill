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
2. (Optional) Create Google OAuth credentials at https://console.cloud.google.com/apis/credentials
   - Callback URL: `http://localhost:3000/auth/google/callback`
3. Add credentials to `.env`
4. Visit `http://localhost:3000/auth/github` or `/auth/google` to login and get a JWT token

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

- **[API Reference](docs/API.md)** — Full endpoint list, pagination, config keys, evaluation
- **[Architecture](docs/ARCHITECTURE.md)** — System design, data flow, retrieval pipeline (with diagrams)
- **[SDK Usage](distill-sdk/README.md)** — Typed Rust client library
- **[Contributing](CONTRIBUTING.md)** — Dev setup, git hooks, code style
- **[Design Document](docs/PLAN.md)** — Original architecture and implementation plan
- **[Eval Fixtures](distill-server/tests/fixtures/)** — Sample datasets for retrieval/contradiction benchmarking

## Tech Stack

- **Language:** Rust
- **Web framework:** Axum
- **Database:** PostgreSQL + pgvector
- **Search:** Hybrid BM25 (tsvector) + vector similarity, merged via RRF
- **LLM:** genai (multi-provider: Gemini, OpenAI, Anthropic, Ollama, etc.)
- **Diff engine:** diffy-imara
- **Auth:** GitHub OAuth + JWT

## Configuration

- **Environment variables** (infrastructure: DB, secrets, model names) — see [`.env.example`](.env.example)
- **Runtime config** (feature toggles, retry attempts, quotas, cache TTL) — stored in DB, managed via `GET/PUT /admin/config`. See [API Reference → Config Keys](docs/API.md#config-keys)

## License

MIT OR Apache-2.0

---

Built with Claude.
