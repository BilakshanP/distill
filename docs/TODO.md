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

## Real-time Features

**Websocket/SSE for live AI answers:**
When a question is submitted and AI answer generation starts, stream progress to the client. TUI could show "generating..." then render the answer as it arrives.

**Notifications:**
Notify users when their questions get new answers, or when contradictions are detected on answers they wrote/rated.

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

---

## Retrieval Improvements

**Cross-encoder reranker:**
Add a reranking stage on top-N fused results. Requires a cross-encoder model (e.g., ms-marco-MiniLM). Significant precision improvement for nuanced queries.

**Title-weighted embeddings:**
Currently `title + body` concatenated equally. Titles are more information-dense for intent matching. Experiment with separate embeddings or prefix-weighting.

**Query understanding layer:**
Feed the LLM rephrase back into actual retrieval (currently it's only shown to the user). Could also add intent classification and query expansion.

---

## Feedback Loop (Biggest Architectural Gap)

The system currently doesn't learn from usage:
- Ratings exist but don't influence ranking
- Query reformulations don't feed back into retrieval
- No click-through tracking or implicit relevance signals

To close this loop:
1. Track which results users click/expand
2. Use ratings as relevance labels for retrieval tuning
3. Feed successful reformulations back as query expansion candidates
4. Periodically retrain/recalibrate RRF weights from user signals

---

## Contradiction Detection

**Confidence scores:**
Ask LLM for a 1-10 confidence score alongside the explanation. Use it to prioritize admin review queue.

**Expanded candidate set:**
Currently capped at 5 nearest answers. Could miss contradictions with distant-but-related questions. Consider tiered approach: top-5 strict, top-20 opportunistic (lower confidence threshold).

---

## Caching

**Content-normalized hashing:**
Current cache key includes full body text — any typo fix invalidates cache. Normalize (lowercase, strip whitespace, etc.) before hashing for more robust cache hits.

---

## Eval & Metrics

**Online evaluation:**
No click-through tracking exists. Add implicit relevance signals (time-on-page, "this helped" buttons) to close the offline/online eval gap.

**Similarity threshold calibration:**
Graph edge threshold (0.7) is hardcoded. Different models have different score distributions. Should be calibrated per-model using the eval harness.

**Token budget accuracy:**
`LENGTH(response)` is a rough proxy for tokens. For production accuracy, use the provider's token counting API or tiktoken-equivalent.
