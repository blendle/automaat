CREATE TABLE global_variables (
    id          Serial  PRIMARY KEY,
    key         VarChar NOT NULL UNIQUE,
    value       Bytea   NOT NULL
);
