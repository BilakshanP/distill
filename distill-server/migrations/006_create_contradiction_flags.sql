CREATE TABLE contradiction_flags (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    answer_id_a UUID NOT NULL REFERENCES answers(id),
    answer_id_b UUID NOT NULL REFERENCES answers(id),
    explanation TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'auto',
    flagged_by UUID REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending',
    reviewed_by UUID REFERENCES users(id),
    detected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at TIMESTAMPTZ
);

CREATE INDEX contradiction_flags_status_idx ON contradiction_flags(status);
