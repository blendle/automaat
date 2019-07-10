ALTER INDEX job_steps_task_id_idx RENAME TO task_steps_task_id_idx;
ALTER INDEX job_steps_pkey RENAME TO task_steps_pkey;
ALTER TYPE JobStepStatus RENAME TO TaskStepStatus;
ALTER TABLE job_steps RENAME COLUMN job_id TO task_id;
ALTER SEQUENCE job_steps_id_seq RENAME TO task_steps_id_seq;
ALTER TABLE job_steps RENAME TO task_steps;

ALTER INDEX jobs_name_pipeline_reference_key RENAME TO tasks_name_pipeline_reference_key;
ALTER INDEX jobs_pipeline_reference_idx RENAME TO tasks_pipeline_reference_idx;
ALTER INDEX jobs_pkey RENAME TO tasks_pkey;
ALTER TYPE JobStatus RENAME TO TaskStatus;
ALTER SEQUENCE jobs_id_seq RENAME TO tasks_id_seq;
ALTER TABLE jobs RENAME TO tasks;
