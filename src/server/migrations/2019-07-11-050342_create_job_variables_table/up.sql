CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE job_variables (
    id          Serial  PRIMARY KEY,
    key         VarChar NOT NULL,
    value       Bytea   NOT NULL,
    job_id      Integer REFERENCES jobs ON DELETE CASCADE
);

CREATE INDEX ON job_variables (job_id);
