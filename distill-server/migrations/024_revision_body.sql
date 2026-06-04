-- Store full body snapshot per revision (diff computed on-the-fly)
ALTER TABLE wiki_answer_edits ADD COLUMN body TEXT NOT NULL DEFAULT '';
