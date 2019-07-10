ALTER TABLE tasks RENAME TO jobs;
ALTER SEQUENCE tasks_id_seq RENAME TO jobs_id_seq;
ALTER TYPE TaskStatus RENAME TO JobStatus;
ALTER INDEX tasks_pkey RENAME TO jobs_pkey;
ALTER INDEX tasks_pipeline_reference_idx RENAME TO jobs_pipeline_reference_idx;
ALTER INDEX tasks_name_pipeline_reference_key RENAME TO jobs_name_pipeline_reference_key;

ALTER TABLE task_steps RENAME TO job_steps;
ALTER SEQUENCE task_steps_id_seq RENAME TO job_steps_id_seq;
ALTER TABLE job_steps RENAME COLUMN task_id TO job_id;
ALTER TYPE TaskStepStatus RENAME TO JobStepStatus;
ALTER INDEX task_steps_pkey RENAME TO job_steps_pkey;
ALTER INDEX task_steps_task_id_idx RENAME TO job_steps_task_id_idx;

