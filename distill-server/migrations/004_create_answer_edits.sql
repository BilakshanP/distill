CREATE TABLE answer_edits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    answer_id UUID NOT NULL REFERENCES answers(id),
    editor_id UUID NOT NULL REFERENCES users(id),
    diff TEXT NOT NULL,
    edit_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX answer_edits_answer_idx ON answer_edits(answer_id, created_at);
