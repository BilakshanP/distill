INSERT INTO config (key, value) VALUES
    ('rater_context_visibility', 'optional')
ON CONFLICT (tenant_id, key) DO NOTHING;
