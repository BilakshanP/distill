-- Individual answers (SO-style, multiple per question)
CREATE TABLE answers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    question_id UUID NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    is_accepted BOOLEAN NOT NULL DEFAULT false,
    tenant_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_answers_question ON answers(question_id, created_at);

-- Ratings on individual answers (star 1-5, one per user per answer)
CREATE TABLE individual_answer_ratings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    answer_id UUID NOT NULL REFERENCES answers(id) ON DELETE CASCADE,
    rater_id UUID NOT NULL REFERENCES users(id),
    score INTEGER NOT NULL CHECK (score >= 1 AND score <= 5),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (answer_id, rater_id)
);

-- Discussions can now also attach to an answer (nullable answer_id)
ALTER TABLE discussions ADD COLUMN answer_id UUID REFERENCES answers(id) ON DELETE CASCADE;
CREATE INDEX idx_discussions_answer ON discussions(answer_id) WHERE answer_id IS NOT NULL;
