CREATE TYPE TaskStatus AS ENUM ('scheduled', 'pending', 'running', 'failed', 'ok');

CREATE TABLE tasks (
    id                 Serial     PRIMARY KEY,
    name               VarChar    NOT NULL,
    description        Text           NULL,
    status             TaskStatus NOT NULL,
    pipeline_reference Integer    REFERENCES pipelines ON DELETE SET NULL,

    UNIQUE(name, pipeline_reference)
);

CREATE INDEX ON tasks (pipeline_reference);
