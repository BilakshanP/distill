CREATE TABLE ratings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    answer_id UUID NOT NULL REFERENCES answers(id),
    rater_id UUID NOT NULL REFERENCES users(id),
    score INTEGER NOT NULL,
    scale_type TEXT NOT NULL DEFAULT '1-5',
    comment TEXT,
    tags TEXT[] NOT NULL DEFAULT '{}',
    rater_original_query TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (answer_id, rater_id)
);

CREATE INDEX ratings_answer_idx ON ratings(answer_id);
