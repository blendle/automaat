CREATE TABLE pipelines (
    id          Serial  PRIMARY KEY,
    name        VarChar NOT NULL UNIQUE,
    description Text        NULL
);
