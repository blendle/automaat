CREATE TABLE sessions (
    id    Serial  PRIMARY KEY,
    token UUID    NOT NULL DEFAULT gen_random_uuid()
);
