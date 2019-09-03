ALTER TABLE variable_advertisements DROP CONSTRAINT variable_advertisements_key_step_id_key;
ALTER TABLE variable_advertisements ADD UNIQUE (step_id);
