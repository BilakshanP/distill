-- Drop old model
DROP TABLE IF EXISTS deep_dives CASCADE;
DROP TABLE IF EXISTS contradiction_flags CASCADE;
DROP TABLE IF EXISTS ratings CASCADE;
DROP TABLE IF EXISTS answer_edits CASCADE;
DROP TABLE IF EXISTS comments CASCADE;
DROP TABLE IF EXISTS answers CASCADE;

-- Wiki answers: one per question, collaboratively edited
CREATE TABLE wiki_answers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    question_id UUID NOT NULL REFERENCES questions(id) ON DELETE CASCADE UNIQUE,
    body TEXT NOT NULL DEFAULT '',
    author_id UUID REFERENCES users(id),
    last_editor_id UUID REFERENCES users(id),
    is_stale BOOLEAN NOT NULL DEFAULT false,
    tenant_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Edit history for wiki answers
CREATE TABLE wiki_answer_edits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wiki_answer_id UUID NOT NULL REFERENCES wiki_answers(id) ON DELETE CASCADE,
    editor_id UUID NOT NULL REFERENCES users(id),
    diff TEXT NOT NULL,
    edit_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Ratings on wiki answers (detailed, same as before)
CREATE TABLE answer_ratings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wiki_answer_id UUID NOT NULL REFERENCES wiki_answers(id) ON DELETE CASCADE,
    rater_id UUID NOT NULL REFERENCES users(id),
    score INTEGER NOT NULL CHECK (score >= 1 AND score <= 10),
    scale_type TEXT NOT NULL DEFAULT 'stars_5',
    comment TEXT,
    tenant_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (wiki_answer_id, rater_id)
);

-- Threaded discussions (Reddit-style, infinite nesting)
CREATE TABLE discussions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    question_id UUID NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES discussions(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    depth INTEGER NOT NULL DEFAULT 0,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    tenant_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_discussions_question ON discussions(question_id, created_at);
CREATE INDEX idx_discussions_parent ON discussions(parent_id);

-- Discussion votes (upvote/downvote, one per user per discussion)
CREATE TABLE discussion_votes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    discussion_id UUID NOT NULL REFERENCES discussions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    direction SMALLINT NOT NULL CHECK (direction IN (-1, 1)),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (discussion_id, user_id)
);

-- Config: synthesis_mode (always, never, manual)
INSERT INTO config (key, value) VALUES ('synthesis_mode', 'manual')
ON CONFLICT (tenant_id, key) DO NOTHING;
