CREATE TABLE question_links (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    question_id_a UUID NOT NULL REFERENCES questions(id),
    question_id_b UUID NOT NULL REFERENCES questions(id),
    link_type TEXT NOT NULL DEFAULT 'related',
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (question_id_a, question_id_b)
);

CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    author_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    question_id UUID REFERENCES questions(id),
    answer_id UUID REFERENCES answers(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (question_id IS NOT NULL OR answer_id IS NOT NULL)
);

CREATE INDEX comments_question_idx ON comments(question_id, created_at);
CREATE INDEX comments_answer_idx ON comments(answer_id, created_at);
