INSERT INTO config (key, value) VALUES
    ('rrf_k', '60'),
    ('rrf_weight_bm25', '1.0'),
    ('rrf_weight_vector', '1.0')
ON CONFLICT (tenant_id, key) DO NOTHING;
