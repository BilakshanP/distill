-- answer_mode now supports: always, never, opt-in, opt-out
UPDATE config SET value = 'opt-in' WHERE key = 'answer_mode' AND value = 'ai-first';
