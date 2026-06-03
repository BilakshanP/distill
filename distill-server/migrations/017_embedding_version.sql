-- Embedding version tracks pipeline changes (model swap, chunking, normalization)
-- Re-embed jobs target questions WHERE embedding_version < current_version
ALTER TABLE questions ADD COLUMN embedding_version INT NOT NULL DEFAULT 0;
ALTER TABLE answers ADD COLUMN embedding_version INT NOT NULL DEFAULT 0;
