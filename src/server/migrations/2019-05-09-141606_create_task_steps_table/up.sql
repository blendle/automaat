CREATE TYPE TaskStepStatus AS ENUM ('initialized', 'pending', 'running', 'failed', 'cancelled', 'ok');

CREATE TABLE task_steps (
    id          Serial         PRIMARY KEY,
    name        VarChar        NOT NULL,
    description Text               NULL,
    processor   Jsonb          NOT NULL,
    position    Integer        NOT NULL,
    started_at  Timestamp          NULL,
    finished_at Timestamp          NULL,
    status      TaskStepStatus NOT NULL DEFAULT 'pending',
    output      Text               NULL,
    task_id     Integer        REFERENCES tasks ON DELETE CASCADE
);

CREATE INDEX ON task_steps (task_id);
