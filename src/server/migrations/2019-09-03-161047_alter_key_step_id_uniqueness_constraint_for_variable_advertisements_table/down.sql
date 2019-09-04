ALTER TABLE variable_advertisements DROP CONSTRAINT variable_advertisements_step_id_key;
ALTER TABLE variable_advertisements ADD UNIQUE (key, step_id);
