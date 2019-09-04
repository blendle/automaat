ALTER TABLE steps DROP CONSTRAINT steps_position_task_id_key;
ALTER TABLE steps ADD UNIQUE (position, task_id);
