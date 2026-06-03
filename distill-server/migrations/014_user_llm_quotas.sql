-- Per-user LLM usage tracking and quotas (optional, admin-configurable)
CREATE TABLE user_llm_usage (
    user_id UUID NOT NULL REFERENCES users(id),
    month DATE NOT NULL DEFAULT date_trunc('month', now())::date,
    request_count INT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, month)
);

-- Admin sets quota per user (NULL = use global default)
ALTER TABLE users ADD COLUMN llm_quota_monthly INT;
