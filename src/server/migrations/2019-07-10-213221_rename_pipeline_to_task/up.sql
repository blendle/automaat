ALTER TABLE pipelines RENAME TO tasks;
ALTER SEQUENCE pipelines_id_seq RENAME TO tasks_id_seq;
ALTER INDEX pipelines_pkey RENAME TO tasks_pkey;
ALTER INDEX pipelines_name_key RENAME TO tasks_name_key;

ALTER TABLE steps RENAME COLUMN pipeline_id TO task_id;
ALTER INDEX steps_position_pipeline_id_key RENAME TO steps_position_task_id_key;
ALTER INDEX steps_pipeline_id_idx RENAME TO steps_task_id_idx;

ALTER TABLE variables RENAME COLUMN pipeline_id TO task_id;
ALTER INDEX variables_pipeline_id_idx RENAME TO variables_task_id_idx;

ALTER TABLE jobs RENAME COLUMN pipeline_reference TO task_reference;
ALTER INDEX jobs_pipeline_reference_idx RENAME TO jobs_task_reference_idx;
ALTER INDEX jobs_name_pipeline_reference_key RENAME TO jobs_name_task_reference_key;
