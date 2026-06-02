INSERT INTO config (key, value) VALUES
    ('llm_features_enabled', 'true'),
    ('rephrase_enabled', 'true'),
    ('dig_deeper_enabled', 'true'),
    ('auto_contradiction_detection', 'true'),
    ('llm_retry_attempts', '3')
ON CONFLICT (tenant_id, key) DO NOTHING;
