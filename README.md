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
9. Visualizes knowledge as an interactive graph

## Quick Start

### Prerequisites

- Rust 1.93+
- Docker (for PostgreSQL + pgvector)
- A Gemini/OpenAI/Anthropic API key (optional for dev, required for AI features)

### Setup

```bash
git clone <repo-url> && cd distill

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

## API Endpoints

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| GET | `/health` | No | Health check |
| GET | `/auth/github` | No | Start OAuth flow |
| GET | `/auth/github/callback` | No | OAuth callback (returns JWT) |
| GET | `/me` | Yes | Current user profile |
| DELETE | `/me` | Yes | Delete account (anonymizes contributions, scrubs PII) |
| POST | `/questions` | Yes | Create a question |
| GET | `/questions/:id` | No | Get a question |
| GET | `/questions/search?q=` | No | Hybrid search (BM25 + vector + RRF) |
| POST | `/questions/preview` | Yes | Preview matches + rephrased query |
| GET | `/questions/:id/answers` | No | Get answers for a question |
| PUT | `/answers/:id` | Yes | Edit an answer (stores diff) |
| GET | `/answers/:id/history` | No | Edit history with unified diffs |
| POST | `/answers/:id/ratings` | Yes | Rate an answer |
| GET | `/answers/:id/ratings` | No | Get all ratings (paginated, with rater context) |
| PUT | `/answers/:id/ratings/redact` | Yes | Redact PII from your own rating |
| POST | `/answers/:id/dig-deeper` | Yes | Ask LLM to elaborate |
| GET | `/answers/:id/deep-dives` | No | Get all deep dives |
| POST | `/answers/:id/mark-stale` | Yes | Mark an answer as stale/deprecated |
| POST | `/answers/:id/flag-contradiction` | Yes | Flag a contradiction |
| GET | `/answers/:id/contradictions` | No | Get contradiction flags |
| GET | `/admin/contradictions` | Admin | Review queue (paginated, pending flags) |
| GET | `/admin/config` | Admin | Get deployment config |
| PUT | `/admin/config` | Admin | Update deployment config |
| GET | `/graph` | No | Knowledge graph (nodes + edges) |
| GET | `/graph/node/:id` | No | Node neighborhood (2-hop subgraph) |

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

MIT
