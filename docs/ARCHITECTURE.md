# Architecture

## System Overview

```mermaid
graph TB
    Client[Client / SDK] --> API[Axum API Server]
    API --> DB[(PostgreSQL + pgvector)]
    API --> LLM[LLM Provider<br/>Gemini / OpenAI / Anthropic]
    API --> JobQueue[Job Queue<br/>PostgreSQL-backed]
    JobQueue --> Worker[Background Worker]
    Worker --> DB
    Worker --> LLM

    subgraph "PostgreSQL"
        DB
        RLS[Row Level Security]
        TSV[tsvector / BM25]
        VEC[pgvector / Cosine]
    end
```

## Request Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant A as API
    participant Auth as AuthUser Extractor
    participant RLS as PostgreSQL RLS
    participant DB as Database

    C->>A: Request + Bearer JWT
    A->>Auth: Extract & validate token
    Auth->>Auth: set_tenant(claims.tenant_id)
    Auth->>RLS: SET app.current_tenant
    A->>DB: Query (RLS filters automatically)
    DB-->>A: Tenant-scoped results
    A-->>C: Response
```

## Retrieval Pipeline

```mermaid
flowchart TD
    Q[Query Text] --> EMB{Embedding<br/>available?}
    EMB -->|Yes| HYBRID[Hybrid Search]
    EMB -->|No / Timeout| KW[Keyword Only]

    HYBRID --> BM25[BM25 via tsvector<br/>top 100]
    HYBRID --> VEC[Vector via pgvector<br/>top 100]
    BM25 --> RRF[Reciprocal Rank Fusion<br/>k=60]
    VEC --> RRF
    RRF --> RESULTS[Ranked Results]

    KW --> BM25_ONLY[BM25 via tsvector<br/>ts_rank]
    BM25_ONLY --> RESULTS

    RESULTS --> RETURN[Return to caller]

    style RRF fill:#f9f,stroke:#333
```

## AI Answer Generation

```mermaid
flowchart TD
    NEW[New Question Created] --> JOB[Enqueue GenerateAiAnswer Job]
    JOB --> WORKER[Worker Picks Up]
    WORKER --> FETCH_EMB[Fetch Question Embedding]
    FETCH_EMB --> HAS_EMB{Has embedding?}
    HAS_EMB -->|Yes| RRF[Hybrid RRF Retrieval<br/>top 5 answered Q&A pairs]
    HAS_EMB -->|No| BM25[BM25 Retrieval<br/>top 5 answered Q&A pairs]
    RRF --> CONTEXT[Build context string]
    BM25 --> CONTEXT
    CONTEXT --> LLM[LLM Call with retry]
    LLM --> INSERT[INSERT answer<br/>RETURNING id]
    INSERT --> CONTRADICT[Trigger Contradiction Detection]
```

## Contradiction Detection

```mermaid
flowchart TD
    ANSWER[New Answer Inserted] --> EMB{Parent question<br/>has embedding?}
    EMB -->|Yes| SEM[Semantic Nearest Neighbors<br/>ORDER BY embedding distance<br/>top 5 answers]
    EMB -->|No| SAME[Same-question answers<br/>top 5 by recency]
    SEM --> LOOP[For each candidate]
    SAME --> LOOP
    LOOP --> CACHE{Cached?}
    CACHE -->|Hit| USE[Use cached result]
    CACHE -->|Miss| LLM[LLM comparison<br/>with retry]
    LLM --> STORE[Store in cache]
    STORE --> CHECK{Contradiction?}
    USE --> CHECK
    CHECK -->|Yes| FLAG[INSERT flag<br/>ON CONFLICT DO NOTHING]
    CHECK -->|No| SKIP[Skip]
```

## Job Queue Lifecycle

```mermaid
stateDiagram-v2
    [*] --> pending: enqueue()
    pending --> running: Worker picks up<br/>(FOR UPDATE SKIP LOCKED)
    running --> completed: Success
    running --> pending: Failure<br/>(attempts < max)<br/>backoff: 4^n seconds
    running --> failed: Failure<br/>(attempts >= max)
    failed --> [*]
    completed --> [*]
```

## Data Model

```mermaid
erDiagram
    TENANTS ||--o{ USERS : has
    USERS ||--o{ QUESTIONS : asks
    USERS ||--o{ ANSWERS : writes
    USERS ||--o{ RATINGS : rates
    USERS ||--o{ COMMENTS : comments
    QUESTIONS ||--o{ ANSWERS : has
    QUESTIONS ||--o{ COMMENTS : has
    QUESTIONS }o--o{ QUESTIONS : linked
    ANSWERS ||--o{ RATINGS : receives
    ANSWERS ||--o{ COMMENTS : has
    ANSWERS ||--o{ ANSWER_EDITS : history
    ANSWERS ||--o{ DEEP_DIVES : explored
    ANSWERS }o--o{ ANSWERS : contradicts

    QUESTIONS {
        uuid id PK
        uuid tenant_id FK
        uuid author_id FK
        text title
        text body
        vector embedding
        text embedding_model
        int embedding_version
        tsvector tsv
        text[] tags
    }

    ANSWERS {
        uuid id PK
        uuid question_id FK
        text author_type
        text body
        bool is_stale
    }

    CONTRADICTION_FLAGS {
        uuid id PK
        uuid answer_id_a FK
        uuid answer_id_b FK
        text explanation
        text source
        text status
    }
```

## Module Structure

```mermaid
graph LR
    subgraph "distill-server"
        MAIN[main.rs] --> LIB[lib.rs<br/>Router + AppState]
        LIB --> ROUTES[routes/]
        LIB --> SERVICES[services/]
        LIB --> JOBS[jobs.rs]
        LIB --> AUTH[auth/]
        LIB --> ERROR[error.rs]

        ROUTES --> |delegates| SERVICES
        ROUTES --> |enqueues| JOBS
        JOBS --> |executes| SERVICES
    end

    subgraph "distill-sdk"
        SDK[client.rs] --> TYPES[types.rs]
    end

    SDK -.->|HTTP| LIB
```

## Multi-Tenancy

```mermaid
flowchart TD
    REQ[Request arrives] --> JWT[JWT contains tenant_id]
    JWT --> EXTRACT[AuthUser extractor]
    EXTRACT --> SET[set_config 'app.current_tenant']
    SET --> RLS[PostgreSQL RLS Policy]
    RLS --> FILTER[Automatic row filtering<br/>on all queries]

    ADMIN[Admin: no tenant_id] --> DEFAULT[COALESCE to default UUID]
    DEFAULT --> RLS
```
