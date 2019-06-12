CREATE TABLE variables (
    id          Serial  PRIMARY KEY,
    key         VarChar NOT NULL,
    description Text        NULL,
    pipeline_id Integer REFERENCES pipelines ON DELETE CASCADE
);

CREATE INDEX ON variables (pipeline_id);
