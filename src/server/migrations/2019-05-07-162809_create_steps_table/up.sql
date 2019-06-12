CREATE TABLE steps (
    id          Serial  PRIMARY KEY,
    name        VarChar NOT NULL,
    description Text        NULL,
    processor   Jsonb   NOT NULL,
    position    Integer NOT NULL,
    pipeline_id Integer REFERENCES pipelines ON DELETE CASCADE,

    UNIQUE(position, pipeline_id)
);

CREATE INDEX ON steps (pipeline_id);
