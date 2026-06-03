-- Prevent duplicate contradiction records between the same pair of answers
CREATE UNIQUE INDEX contradiction_flags_pair_idx
ON contradiction_flags (LEAST(answer_id_a, answer_id_b), GREATEST(answer_id_a, answer_id_b));
