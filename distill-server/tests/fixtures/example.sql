-- Seed data for Distill Q&A platform
-- Generated: 2026-06-04

-- ============================================================
-- USERS (8 users: 1 admin, 7 regular)
-- ============================================================
INSERT INTO users (id, provider, provider_id, display_name, email, avatar_url, role, created_at) VALUES
('a0000000-0000-0000-0000-000000000001', 'github', '10001', 'Kira Nakamura', 'kira@distill.dev', 'https://avatars.githubusercontent.com/u/10001', 'admin', now() - interval '30 days'),
('a0000000-0000-0000-0000-000000000002', 'github', '10002', 'Marcus Chen', 'marcus.chen@example.com', 'https://avatars.githubusercontent.com/u/10002', 'user', now() - interval '28 days'),
('a0000000-0000-0000-0000-000000000003', 'github', '10003', 'Priya Sharma', 'priya.sharma@example.com', 'https://avatars.githubusercontent.com/u/10003', 'user', now() - interval '27 days'),
('a0000000-0000-0000-0000-000000000004', 'github', '10004', 'Jordan Ellis', 'jordan.ellis@example.com', 'https://avatars.githubusercontent.com/u/10004', 'user', now() - interval '25 days'),
('a0000000-0000-0000-0000-000000000005', 'google', '20001', 'Alexei Volkov', 'alexei.volkov@example.com', 'https://avatars.githubusercontent.com/u/20001', 'user', now() - interval '24 days'),
('a0000000-0000-0000-0000-000000000006', 'github', '10006', 'Sam Okafor', 'sam.okafor@example.com', 'https://avatars.githubusercontent.com/u/10006', 'user', now() - interval '22 days'),
('a0000000-0000-0000-0000-000000000007', 'github', '10007', 'Lena Bergström', 'lena.bergstrom@example.com', 'https://avatars.githubusercontent.com/u/10007', 'user', now() - interval '20 days'),
('a0000000-0000-0000-0000-000000000008', 'google', '20002', 'Dev Patel', 'dev.patel@example.com', 'https://avatars.githubusercontent.com/u/20002', 'user', now() - interval '18 days');

-- ============================================================
-- QUESTIONS (45 questions across multiple topics)
-- ============================================================
INSERT INTO questions (author_id, title, body, original_query, tags, status, created_at) VALUES
-- Rust (questions 1-6)
('a0000000-0000-0000-0000-000000000002', 'How do I handle lifetimes in async Rust functions?', 'I''m building an async web server and keep running into lifetime errors when passing references across .await points. The compiler says my reference doesn''t live long enough but I can''t figure out what owns what.', 'rust async lifetime errors', '{rust,async,lifetimes}', 'open', now() - interval '29 days'),
('a0000000-0000-0000-0000-000000000003', 'What is the best way to implement the Builder pattern in Rust?', 'I want to create a complex configuration struct with many optional fields. I''ve seen typestate builders and Option-based builders but I''m not sure which approach is more idiomatic for library APIs.', 'rust builder pattern best practice', '{rust,design-patterns}', 'open', now() - interval '28 days'),
('a0000000-0000-0000-0000-000000000004', 'When should I use Arc<Mutex<T>> vs channels for shared state?', 'I have multiple tokio tasks that need to read and write a shared HashMap. Performance matters since this is on the hot path. Should I use Arc<Mutex<T>>, Arc<RwLock<T>>, or switch to channels entirely?', 'rust shared state tokio arc mutex vs channels', '{rust,concurrency,tokio}', 'open', now() - interval '27 days'),
('a0000000-0000-0000-0000-000000000005', 'How to properly propagate errors with thiserror and anyhow?', 'My application has library code and binary code. I''ve heard thiserror is for libraries and anyhow is for applications, but my error types are getting unwieldy. What''s a clean architecture for error handling across crate boundaries?', 'rust error handling thiserror anyhow', '{rust,error-handling}', 'open', now() - interval '25 days'),
('a0000000-0000-0000-0000-000000000006', 'Zero-copy deserialization with serde: when is it worth it?', 'I''m parsing large JSON payloads (50MB+) and considering using serde with borrowing deserializers. The docs mention Cow<str> and #[serde(borrow)] but I''m not sure when the complexity is justified vs just using owned Strings.', 'serde zero copy deserialization performance', '{rust,serde,performance}', 'open', now() - interval '22 days'),
('a0000000-0000-0000-0000-000000000007', 'How do I write a procedural macro that generates impl blocks?', 'I want to auto-generate trait implementations based on struct field attributes. I''ve read about proc-macro2 and syn but the token stream manipulation feels overwhelming. Any tips for a structured approach?', 'rust proc macro derive implementation', '{rust,macros,metaprogramming}', 'open', now() - interval '18 days'),

-- PostgreSQL (questions 7-12)
('a0000000-0000-0000-0000-000000000003', 'How to optimize a slow query with multiple JOINs and a WHERE on jsonb?', 'I have a query joining 4 tables with a GIN-indexed jsonb column in the WHERE clause. EXPLAIN shows a seq scan on the largest table (2M rows) despite the index. Query takes 3+ seconds.', 'postgresql slow query jsonb gin index not used', '{postgresql,performance,indexing}', 'open', now() - interval '28 days'),
('a0000000-0000-0000-0000-000000000004', 'What is the correct way to use pgvector for hybrid search?', 'I want to combine full-text search (tsvector) with vector similarity search (pgvector) and merge results using Reciprocal Rank Fusion. Should I do this in one query or two separate queries merged in application code?', 'pgvector hybrid search rrf tsvector', '{postgresql,pgvector,search}', 'open', now() - interval '26 days'),
('a0000000-0000-0000-0000-000000000005', 'Row-level security vs application-level tenant filtering?', 'We''re building a multi-tenant SaaS and debating between PostgreSQL RLS policies and filtering in our ORM. RLS seems more secure but adds complexity to migrations and debugging. What are the real-world tradeoffs?', 'postgresql row level security multi tenant', '{postgresql,security,multi-tenancy}', 'open', now() - interval '24 days'),
('a0000000-0000-0000-0000-000000000008', 'How to handle schema migrations without downtime?', 'Our production database serves traffic 24/7 and we need to add columns, create indexes, and rename tables without locking. We use sqlx for migrations. What patterns avoid blocking writes?', 'postgresql zero downtime migrations online ddl', '{postgresql,migrations,devops}', 'open', now() - interval '21 days'),
('a0000000-0000-0000-0000-000000000002', 'Partitioning a time-series table: range vs hash?', 'We have an events table growing by 10M rows/day. Queries mostly filter by created_at ranges. Should we use declarative range partitioning by month, or would hash partitioning give better write distribution?', 'postgresql partitioning time series range hash', '{postgresql,partitioning,time-series}', 'open', now() - interval '19 days'),
('a0000000-0000-0000-0000-000000000006', 'Understanding MVCC bloat and when to tune autovacuum', 'Our table has high update frequency and pg_stat_user_tables shows a huge dead tuple count. Autovacuum seems to run but can''t keep up. How do we tune autovacuum_vacuum_scale_factor and related settings?', 'postgresql mvcc bloat autovacuum tuning dead tuples', '{postgresql,vacuum,performance}', 'open', now() - interval '15 days'),

-- System Design (questions 13-18)
('a0000000-0000-0000-0000-000000000007', 'How to design a rate limiter for a distributed API gateway?', 'We need per-user and per-endpoint rate limiting across 12 API gateway instances. Token bucket seems right but synchronizing state across instances is the hard part. Redis vs local sliding window with gossip?', 'distributed rate limiter design api gateway', '{system-design,distributed-systems,api}', 'open', now() - interval '27 days'),
('a0000000-0000-0000-0000-000000000002', 'Event sourcing vs CRUD: when does the complexity pay off?', 'I''m designing an audit-heavy financial system. Event sourcing gives us a perfect audit trail but adds complexity with projections, snapshots, and eventual consistency. For a team of 4, is it worth it?', 'event sourcing vs crud when to use', '{system-design,architecture,event-sourcing}', 'open', now() - interval '25 days'),
('a0000000-0000-0000-0000-000000000004', 'Designing a notification system that scales to millions of users', 'We need to support push notifications, email, SMS, and in-app notifications with user preferences, batching, and delivery guarantees. Current design using a single queue is hitting limits at 100k users.', 'notification system design scale millions', '{system-design,scalability,messaging}', 'open', now() - interval '23 days'),
('a0000000-0000-0000-0000-000000000005', 'CQRS with separate read/write databases: handling eventual consistency', 'We split our write model (PostgreSQL) from read model (Elasticsearch) but users complain about stale reads after writes. The replication lag is 200-500ms. What patterns handle this gracefully in the UI?', 'cqrs eventual consistency read after write', '{system-design,cqrs,consistency}', 'open', now() - interval '20 days'),
('a0000000-0000-0000-0000-000000000003', 'How to design a reliable webhook delivery system?', 'I need to deliver webhooks with at-least-once semantics, exponential backoff, and a dead letter queue. The system should handle 10k events/second and let consumers query delivery status. What''s the right architecture?', 'webhook delivery system reliable design', '{system-design,webhooks,reliability}', 'open', now() - interval '16 days'),
('a0000000-0000-0000-0000-000000000008', 'Circuit breaker pattern: when to open, half-open, and close?', 'I''m implementing a circuit breaker for our external payment provider calls. What thresholds make sense for error rate, timeout, and half-open probe count? We process ~500 requests/second.', 'circuit breaker pattern thresholds design', '{system-design,resilience,patterns}', 'open', now() - interval '12 days'),

-- DevOps (questions 19-24)
('a0000000-0000-0000-0000-000000000006', 'How to structure a monorepo CI/CD pipeline with selective builds?', 'We have 8 Rust crates in a workspace and want to only rebuild/test crates affected by a PR. GitHub Actions is our CI. Is there a reliable way to detect which crates changed and skip the rest?', 'monorepo ci selective builds github actions rust', '{devops,ci-cd,monorepo}', 'open', now() - interval '26 days'),
('a0000000-0000-0000-0000-000000000007', 'Docker layer caching strategy for Rust builds?', 'Our Rust Docker builds take 15+ minutes because cargo re-downloads and recompiles all dependencies on every change. I''ve tried cargo-chef but the cache invalidation seems fragile. What''s the most reliable approach?', 'docker rust build cache cargo chef layers', '{devops,docker,rust,performance}', 'open', now() - interval '24 days'),
('a0000000-0000-0000-0000-000000000002', 'Terraform vs Pulumi for a small team managing AWS infrastructure?', 'We''re 3 developers who need to manage ECS, RDS, S3, and CloudFront. Terraform is battle-tested but HCL feels limiting. Pulumi lets us use real languages. For a team our size, which has less operational overhead?', 'terraform vs pulumi small team aws', '{devops,iac,aws}', 'open', now() - interval '22 days'),
('a0000000-0000-0000-0000-000000000008', 'How to implement blue-green deployments on ECS Fargate?', 'We want zero-downtime deploys with instant rollback capability. CodeDeploy with ECS seems like the standard approach but the configuration is complex. Is there a simpler pattern using just ALB target groups?', 'ecs fargate blue green deployment codedeploy', '{devops,aws,ecs,deployment}', 'open', now() - interval '19 days'),
('a0000000-0000-0000-0000-000000000004', 'Secrets management: Vault vs AWS Secrets Manager vs SOPS?', 'We currently have secrets in .env files and want to centralize. HashiCorp Vault seems overkill for our 5-service setup. AWS Secrets Manager costs per secret. SOPS encrypts in git. What''s pragmatic for a startup?', 'secrets management vault aws sops comparison', '{devops,security,secrets}', 'open', now() - interval '17 days'),
('a0000000-0000-0000-0000-000000000003', 'Observability stack: OpenTelemetry + Grafana vs Datadog?', 'We need traces, metrics, and logs correlated together. Datadog is expensive but integrated. Self-hosted Grafana stack (Tempo, Mimir, Loki) is cheaper but requires maintenance. What''s the break-even point?', 'observability opentelemetry grafana vs datadog cost', '{devops,observability,monitoring}', 'open', now() - interval '14 days'),

-- Web Development (questions 25-30)
('a0000000-0000-0000-0000-000000000005', 'How to implement proper JWT refresh token rotation?', 'I have short-lived access tokens (15 min) and long-lived refresh tokens (7 days). When a refresh token is used, should I issue a new refresh token? How do I handle the race condition of concurrent requests using the same refresh token?', 'jwt refresh token rotation concurrent requests', '{web,security,authentication}', 'open', now() - interval '27 days'),
('a0000000-0000-0000-0000-000000000006', 'Server-Sent Events vs WebSockets for real-time notifications?', 'Our app needs to push notifications to users in real-time. The data flow is strictly server-to-client. SSE seems simpler but I''m worried about connection limits and proxy compatibility. Is SSE sufficient or should I use WebSockets?', 'sse vs websockets real-time notifications', '{web,real-time,architecture}', 'open', now() - interval '23 days'),
('a0000000-0000-0000-0000-000000000007', 'Implementing pagination: cursor-based vs offset with total count?', 'Our API returns large result sets and clients need stable pagination. Offset pagination breaks when items are inserted/deleted. Cursor pagination is stable but makes "jump to page N" impossible. How do real APIs handle this?', 'api pagination cursor vs offset tradeoffs', '{web,api-design,pagination}', 'open', now() - interval '21 days'),
('a0000000-0000-0000-0000-000000000002', 'How to handle file uploads with presigned URLs and progress tracking?', 'Users upload files up to 500MB. We want to avoid proxying through our server. The plan is presigned S3 URLs, but how do we track upload progress and handle resumable uploads for flaky connections?', 'file upload presigned url s3 progress resumable', '{web,aws,file-upload}', 'open', now() - interval '18 days'),
('a0000000-0000-0000-0000-000000000008', 'CORS configuration for a multi-subdomain SPA with API on a different domain', 'Our SPA is on app.example.com, API on api.example.com, and we have white-label domains. Dynamic CORS origin validation is needed but I want to avoid security holes. What''s the safest pattern?', 'cors multi domain spa api configuration security', '{web,security,cors}', 'open', now() - interval '14 days'),
('a0000000-0000-0000-0000-000000000004', 'Structuring Axum extractors for clean request validation?', 'I want to validate request bodies, query params, and path params in Axum with custom error responses. Should I use a custom extractor, a middleware layer, or tower-http validators? Looking for the most maintainable approach.', 'axum extractors validation custom errors', '{web,rust,axum}', 'open', now() - interval '10 days'),

-- AI/ML (questions 31-36)
('a0000000-0000-0000-0000-000000000003', 'How to evaluate RAG retrieval quality without labeled test sets?', 'We built a RAG pipeline but don''t have ground-truth relevance labels. I''ve seen mention of LLM-as-judge and synthetic test generation. What metrics should we track and how do we bootstrap an evaluation dataset?', 'rag evaluation retrieval quality metrics llm judge', '{ai-ml,rag,evaluation}', 'open', now() - interval '26 days'),
('a0000000-0000-0000-0000-000000000005', 'Chunking strategies for long documents in a vector search pipeline?', 'We''re indexing technical documentation (5k-50k tokens per doc) into pgvector. Fixed-size chunks miss context, semantic chunking is expensive. What chunk sizes and overlap work well in practice for Q&A retrieval?', 'document chunking vector search rag strategy', '{ai-ml,rag,vector-search}', 'open', now() - interval '23 days'),
('a0000000-0000-0000-0000-000000000006', 'How to detect and handle LLM hallucinations in production?', 'Our AI generates answers from retrieved context but sometimes confabulates facts not in the source material. We need a reliable way to flag potential hallucinations before showing answers to users. What techniques actually work?', 'llm hallucination detection production grounding', '{ai-ml,llm,reliability}', 'open', now() - interval '20 days'),
('a0000000-0000-0000-0000-000000000008', 'Fine-tuning vs few-shot prompting vs RAG: decision framework?', 'We have domain-specific knowledge that the base model doesn''t know. Budget is limited and we need fast iteration. How do I decide between fine-tuning a smaller model, elaborate few-shot prompts, or building a RAG system?', 'fine tuning vs few shot vs rag when to use', '{ai-ml,llm,architecture}', 'open', now() - interval '17 days'),
('a0000000-0000-0000-0000-000000000002', 'Implementing streaming LLM responses with backpressure?', 'Our API streams token-by-token from the LLM to the client via SSE. When clients are slow, tokens buffer in memory. How do I implement backpressure so we don''t OOM when many users stream concurrently?', 'llm streaming backpressure sse memory', '{ai-ml,streaming,performance}', 'open', now() - interval '13 days'),
('a0000000-0000-0000-0000-000000000007', 'Embedding model selection: OpenAI ada-002 vs open-source alternatives?', 'We need embeddings for semantic search over 500k documents. OpenAI embeddings are easy but add latency and cost. Are models like E5-large or BGE competitive enough for production Q&A search?', 'embedding model comparison openai vs open source', '{ai-ml,embeddings,vector-search}', 'open', now() - interval '9 days'),

-- Security (questions 37-41)
('a0000000-0000-0000-0000-000000000004', 'How to implement RBAC with resource-level permissions in a REST API?', 'We need role-based access control where some users can edit specific resources but not others. A flat role system is too coarse. How do I model per-resource permissions without making every endpoint check 10 different things?', 'rbac resource level permissions rest api design', '{security,authorization,api-design}', 'open', now() - interval '25 days'),
('a0000000-0000-0000-0000-000000000005', 'Preventing SSRF in a user-provided webhook URL feature?', 'Users can register webhook URLs and our server makes HTTP calls to them. We need to prevent SSRF attacks where users provide internal network addresses. DNS rebinding makes simple blocklists insufficient. What''s a robust defense?', 'ssrf prevention webhook url validation', '{security,web,ssrf}', 'open', now() - interval '21 days'),
('a0000000-0000-0000-0000-000000000003', 'Content Security Policy for an app that loads user-generated content?', 'Our app displays user-submitted HTML (sanitized) and loads images from arbitrary domains. CSP needs to be strict enough to prevent XSS but permissive enough for functionality. How do I balance this?', 'csp content security policy user generated content', '{security,web,csp}', 'open', now() - interval '16 days'),
('a0000000-0000-0000-0000-000000000007', 'How to audit and rotate compromised API keys without downtime?', 'We discovered a leaked API key in a public repo. We need to rotate it immediately but 200+ clients use it. How do we implement graceful key rotation with overlap periods and client notification?', 'api key rotation compromised graceful migration', '{security,devops,incident-response}', 'open', now() - interval '11 days'),
('a0000000-0000-0000-0000-000000000008', 'SQL injection prevention in dynamic query builders?', 'We build queries dynamically based on user-provided filter criteria. Parameterized queries work for values but what about dynamic column names and sort orders? How do I safely construct these queries?', 'sql injection dynamic query builder prevention', '{security,postgresql,web}', 'open', now() - interval '7 days'),

-- Distributed Systems (questions 42-45)
('a0000000-0000-0000-0000-000000000006', 'How does Raft consensus handle network partitions in practice?', 'I understand the theory of Raft leader election but I''m confused about split-brain scenarios. If a partition heals and there were two leaders, how does the cluster reconcile conflicting log entries?', 'raft consensus network partition split brain', '{distributed-systems,consensus,raft}', 'open', now() - interval '24 days'),
('a0000000-0000-0000-0000-000000000002', 'Idempotency keys: client-generated vs server-generated?', 'We''re adding idempotency to our payment API. Should the client generate the idempotency key (UUID) or should the server derive it from request content? Client-generated is simpler but susceptible to misuse.', 'idempotency key design api payments', '{distributed-systems,api-design,reliability}', 'open', now() - interval '19 days'),
('a0000000-0000-0000-0000-000000000004', 'Saga pattern vs 2PC for distributed transactions across microservices?', 'We have an order service, payment service, and inventory service that need to coordinate. Two-phase commit seems simpler conceptually but I''ve heard it doesn''t scale. When should I use sagas with compensating transactions instead?', 'saga pattern vs two phase commit microservices', '{distributed-systems,transactions,microservices}', 'open', now() - interval '15 days'),
('a0000000-0000-0000-0000-000000000005', 'Consistent hashing for a custom distributed cache: virtual nodes needed?', 'I''m building a distributed cache layer (like memcached) and implementing consistent hashing for key distribution. With 5 nodes, load distribution is uneven. How many virtual nodes per physical node is typical and why?', 'consistent hashing virtual nodes distributed cache', '{distributed-systems,caching,algorithms}', 'open', now() - interval '8 days');

-- ============================================================
-- WIKI ANSWERS (one per question, first 20 questions get answers)
-- ============================================================
INSERT INTO wiki_answers (question_id, body, author_id, last_editor_id, created_at, updated_at)
SELECT q.id,
  'This is the community-maintained answer for: ' || q.title || E'\n\n' ||
  'Key points:' || E'\n' ||
  '- The approach depends on your specific constraints' || E'\n' ||
  '- Consider trade-offs between simplicity and performance' || E'\n' ||
  '- See the discussion below for detailed perspectives',
  ('a0000000-0000-0000-0000-00000000000' || (1 + (row_number() OVER ()) % 8))::uuid,
  ('a0000000-0000-0000-0000-00000000000' || (1 + (row_number() OVER () + 3) % 8))::uuid,
  q.created_at + interval '1 day',
  q.created_at + interval '3 days'
FROM questions q
ORDER BY q.created_at
LIMIT 20;

-- ============================================================
-- DISCUSSIONS (threaded comments, ~80 total)
-- ============================================================
-- Top-level discussions (40)
INSERT INTO discussions (question_id, author_id, body, depth, created_at)
SELECT q.id,
  ('a0000000-0000-0000-0000-00000000000' || (1 + (row_number() OVER ()) % 8))::uuid,
  (ARRAY[
    'I ran into this exact issue last week. The key insight is to think about ownership first, then lifetimes follow naturally.',
    'Great question. In my experience the pragmatic choice depends heavily on team size and deployment constraints.',
    'We benchmarked both approaches and found the simpler solution was only 5% slower — not worth the complexity.',
    'The docs are misleading here. What actually works in production is quite different from the textbook answer.',
    'This is one of those "it depends" situations. Let me share what worked for our 10k RPS service.',
    'I would strongly recommend starting with the simpler approach and only optimizing if you hit measurable bottlenecks.',
    'We tried three different approaches over 6 months. Here is what we learned the hard way.',
    'The official recommendation changed in the latest version. Make sure you are reading current docs.',
    'Hot take: most teams over-engineer this. A simple solution with good monitoring beats a complex one without.',
    'Adding context: we have a similar setup in production handling 50M requests/day and this pattern holds up.'
  ])[1 + (row_number() OVER ()) % 10],
  0,
  q.created_at + interval '2 hours' + (interval '1 hour' * (row_number() OVER () % 24))
FROM questions q
ORDER BY q.created_at
LIMIT 40;

-- Replies to discussions (40)
INSERT INTO discussions (question_id, parent_id, author_id, body, depth, created_at)
SELECT d.question_id,
  d.id,
  ('a0000000-0000-0000-0000-00000000000' || (1 + (row_number() OVER () + 4) % 8))::uuid,
  (ARRAY[
    'Totally agree. We had the same experience and switched to this approach 6 months ago.',
    'Could you elaborate on the performance numbers? What was your test setup?',
    'This contradicts what I have seen. In our case the opposite was true due to network latency.',
    'Thanks for sharing. One thing to add: make sure you handle the edge case of empty inputs.',
    'We use a slightly modified version of this that also accounts for concurrent access patterns.',
    'Good point but this only applies if you are running on a single node. Distributed setups differ.',
    'I tried this and it works. One gotcha: you need to set the timeout higher than the default.',
    'Interesting perspective. Do you have a link to the benchmarks you mentioned?',
    'This is the correct answer IMO. I wish the documentation was this clear.',
    'Disagree slightly — in high-throughput scenarios the overhead becomes noticeable.'
  ])[1 + (row_number() OVER ()) % 10],
  1,
  d.created_at + interval '30 minutes' + (interval '1 hour' * (row_number() OVER () % 12))
FROM discussions d
WHERE d.depth = 0
ORDER BY d.created_at
LIMIT 40;

-- ============================================================
-- DISCUSSION VOTES (120 votes spread across discussions)
-- ============================================================
INSERT INTO discussion_votes (discussion_id, user_id, direction, created_at)
SELECT d.id,
  ('a0000000-0000-0000-0000-00000000000' || (1 + (row_number() OVER ()) % 8))::uuid,
  CASE WHEN random() > 0.2 THEN 1 ELSE -1 END,
  d.created_at + interval '1 hour' * (1 + (row_number() OVER () % 48))
FROM discussions d
ORDER BY random()
LIMIT 120;

-- ============================================================
-- ANSWER RATINGS (30 ratings on wiki answers)
-- ============================================================
INSERT INTO answer_ratings (wiki_answer_id, rater_id, score, scale_type, comment, created_at)
SELECT w.id,
  ('a0000000-0000-0000-0000-00000000000' || (2 + (row_number() OVER ()) % 7))::uuid,
  1 + abs(hashtext(w.id::text || (row_number() OVER ())::text)) % 5,
  'stars_5',
  (ARRAY['Very helpful', 'Good explanation', 'Could be more detailed', 'Exactly what I needed', 'Clear and concise'])[1 + abs(hashtext(w.id::text)) % 5],
  w.created_at + interval '2 days' + (interval '1 day' * (row_number() OVER () % 10))
FROM wiki_answers w
ORDER BY w.created_at
LIMIT 30;

-- ============================================================
-- INDIVIDUAL ANSWERS (30 answers across questions)
-- ============================================================
INSERT INTO answers (question_id, author_id, body, created_at, updated_at)
SELECT q.id,
  ('a0000000-0000-0000-0000-00000000000' || (1 + (row_number() OVER ()) % 8))::uuid,
  (ARRAY[
    'In my experience, the best approach is to start simple and iterate. We ran into this exact problem and found that the naive solution worked for 90% of cases.',
    'I have been using this pattern in production for 2 years now. The key insight is to separate the concerns early — it pays off massively at scale.',
    'The official docs are misleading here. What actually works is to use the builder pattern with a validation step at the end. Here is a minimal example that compiles.',
    'We benchmarked this extensively. The TL;DR is: go with the simpler approach unless you are processing >10k requests/second. Below that threshold the difference is noise.',
    'This is a common misconception. The real issue is not performance but correctness. You need to handle the edge case where the connection drops mid-transaction.',
    'After trying 3 different approaches over 6 months, we settled on this: use a lightweight wrapper that handles retries transparently. Code is much cleaner now.',
    'Hot take: most teams over-engineer this. Start with a simple mutex, add metrics, and only optimize when the numbers tell you to. Premature optimization is real.',
    'The answer depends on your consistency requirements. If you can tolerate eventual consistency, the async approach is 10x simpler. If not, you need distributed locks.',
    'We migrated from the old approach to this one last quarter. Migration was painless — took 2 sprints. Performance improved 40% and code complexity dropped significantly.',
    'I wrote a blog post about this exact topic. The short version: use the type system to make invalid states unrepresentable. The compiler becomes your safety net.'
  ])[1 + (row_number() OVER ()) % 10],
  q.created_at + interval '4 hours' + (interval '2 hours' * (row_number() OVER () % 12)),
  q.created_at + interval '4 hours' + (interval '2 hours' * (row_number() OVER () % 12))
FROM questions q
ORDER BY q.created_at
LIMIT 30;

-- ============================================================
-- INDIVIDUAL ANSWER RATINGS (50 ratings)
-- ============================================================
INSERT INTO individual_answer_ratings (answer_id, rater_id, score, created_at)
SELECT a.id,
  ('a0000000-0000-0000-0000-00000000000' || (2 + (row_number() OVER ()) % 7))::uuid,
  1 + abs(hashtext(a.id::text || (row_number() OVER ())::text)) % 5,
  a.created_at + interval '1 day' + (interval '6 hours' * (row_number() OVER () % 8))
FROM answers a
ORDER BY a.created_at
LIMIT 50;

-- ============================================================
-- DISCUSSIONS ON ANSWERS (20 per-answer threads)
-- ============================================================
INSERT INTO discussions (question_id, answer_id, author_id, body, depth, created_at)
SELECT a.question_id,
  a.id,
  ('a0000000-0000-0000-0000-00000000000' || (1 + (row_number() OVER () + 3) % 8))::uuid,
  (ARRAY[
    'This worked for us too. One addition: make sure to handle the timeout case explicitly.',
    'Could you clarify what you mean by "simple wrapper"? Do you mean a newtype or a trait object?',
    'We tried this but hit issues with lifetimes across await points. Any tips?',
    'Solid answer. I would add that you should also consider the testing story — mocking this is non-trivial.',
    'Disagree slightly. In our case the simple approach caused a production outage due to race conditions.',
    'Thanks for the concrete numbers. That matches what we saw in our load tests.',
    '+1 on using the type system. We encode state machines in types and it catches so many bugs at compile time.',
    'The blog post link would be super helpful if you can share it.',
    'How does this interact with connection pooling? We use deadpool-postgres.',
    'Great answer but I think you are missing the distributed case where network partitions are possible.'
  ])[1 + (row_number() OVER ()) % 10],
  0,
  a.created_at + interval '6 hours' + (interval '3 hours' * (row_number() OVER () % 8))
FROM answers a
ORDER BY random()
LIMIT 20;
