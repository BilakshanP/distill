-- Track which model produced each embedding (prevents silent cross-model corruption)
ALTER TABLE questions ADD COLUMN embedding_model TEXT;
ALTER TABLE answers ADD COLUMN embedding_model TEXT;
