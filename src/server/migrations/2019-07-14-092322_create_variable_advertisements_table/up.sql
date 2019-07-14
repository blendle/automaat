CREATE TABLE variable_advertisements (
    id          Serial  PRIMARY KEY,
    key         VarChar NOT NULL,
    step_id     Integer REFERENCES steps ON DELETE CASCADE,

    UNIQUE(key, step_id)
);

CREATE INDEX ON variable_advertisements (step_id);
