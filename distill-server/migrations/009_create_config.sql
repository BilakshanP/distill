CREATE TABLE config (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, key)
);

INSERT INTO config (key, value) VALUES
    ('rating_scale', '1-5'),
    ('answer_mode', 'ai-first'),
    ('search_mode', 'hybrid'),
    ('live_suggestions', 'false'),
    ('external_sources', 'false'),
    ('graph_visibility', 'public'),
    ('stale_auto_resolve', 'false'),
    ('llm_cache_ttl_hours', '168'),
    ('token_budget_monthly', '');
