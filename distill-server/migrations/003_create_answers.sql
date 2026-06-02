CREATE TABLE answers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    question_id UUID NOT NULL REFERENCES questions(id),
    author_id UUID REFERENCES users(id),
    author_type TEXT NOT NULL DEFAULT 'human',
    body TEXT NOT NULL,
    embedding vector(1536),
    is_stale BOOLEAN NOT NULL DEFAULT false,
    stale_reason TEXT,
    stale_marked_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX answers_question_idx ON answers(question_id);
CREATE INDEX answers_embedding_idx ON answers USING hnsw (embedding vector_cosine_ops);
