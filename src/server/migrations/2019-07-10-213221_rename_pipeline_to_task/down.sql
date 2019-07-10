ALTER INDEX jobs_name_task_reference_key RENAME TO jobs_name_pipeline_reference_key;
ALTER INDEX jobs_task_reference_idx RENAME TO jobs_pipeline_reference_idx;
ALTER TABLE jobs RENAME COLUMN task_reference TO pipeline_reference;

ALTER INDEX variables_task_id_idx RENAME TO variables_pipeline_id_idx;
ALTER TABLE variables RENAME COLUMN task_id TO pipeline_id;

ALTER INDEX steps_task_id_idx RENAME TO steps_pipeline_id_idx;
ALTER INDEX steps_position_task_id_key RENAME TO steps_position_pipeline_id_key;
ALTER TABLE steps RENAME COLUMN task_id TO pipeline_id;

ALTER INDEX tasks_name_key RENAME TO pipelines_name_key;
ALTER INDEX tasks_pkey RENAME TO pipelines_pkey;
ALTER SEQUENCE tasks_id_seq RENAME TO pipelines_id_seq;
ALTER TABLE tasks RENAME TO pipelines;
