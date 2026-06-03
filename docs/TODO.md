# TODO

Future work. None of these are blocking — they're optimizations and features to revisit once Distill has real usage and data.

---

## HNSW Vector Index

**When:** Search p95 latency is measurably high AND vector scan dominates query time AND you have 100k+ questions.

**Not now.** Sequential scan is fine for <100k rows. Current latency is likely dominated by embedding API calls (300ms), not Postgres queries (15ms).

The architecture already supports it:
- `embedding_model TEXT` tracks provenance
- `embedding_version INT` enables re-embedding
- Dimensionless `vector` column allows model swaps

When the time comes (after settling on a model with real traffic data):
```sql
ALTER TABLE questions ALTER COLUMN embedding TYPE vector(768);
CREATE INDEX questions_embedding_idx ON questions USING hnsw (embedding vector_cosine_ops);
```

One migration. Not an architecture problem.

---

## Load Testing

**When:** Before production launch or when expecting >100 concurrent users.

Tools: k6, wrk, or hey.

What to measure:
- Search p50/p95/p99 latency
- Question creation throughput
- Job queue drain rate under load
- Rate limiter behavior at boundary

---

## Larger Eval Datasets

**When:** After populating the DB with real questions (50+ labeled pairs minimum).

Current eval harnesses exist (`cargo run --bin eval`, `cargo run --bin eval_contradictions`) but sample datasets only have 2-5 cases. Real evaluation needs:
- 50+ retrieval queries with ground-truth relevant IDs
- 50+ contradiction pairs (balanced true/false)
- Periodic re-runs after model or parameter changes

---

## CLI Client

Build a terminal client for interacting with Distill without a browser. Could use the SDK directly.

---

## Web UI

Frontend for browsing questions, submitting, viewing the knowledge graph, admin dashboard.

---

## Production Deployment

- Dockerfile + multi-stage build
- Helm chart or docker-compose for production
- Secrets management (Vault, AWS SSM, etc.)
- Log aggregation
- Health check endpoint already exists (`GET /health`)
- Metrics/tracing export (OpenTelemetry)
