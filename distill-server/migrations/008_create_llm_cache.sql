CREATE TABLE llm_cache (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    cache_key TEXT NOT NULL UNIQUE,
    operation_type TEXT NOT NULL,
    response TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX llm_cache_key_idx ON llm_cache(cache_key);
CREATE INDEX llm_cache_expires_idx ON llm_cache(expires_at);
