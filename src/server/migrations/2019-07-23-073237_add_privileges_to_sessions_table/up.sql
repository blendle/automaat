ALTER TABLE sessions ADD COLUMN privileges Text[] NOT NULL DEFAULT '{}';

CREATE FUNCTION automaat_validate_privilege(txt Text[]) RETURNS boolean AS $$
    SELECT bool_and (str ~ '^[a-z0-9_]+$') FROM unnest(txt) s(str);
$$ IMMUTABLE STRICT LANGUAGE SQL;

ALTER TABLE sessions ADD CONSTRAINT privilege_syntax CHECK (automaat_validate_privilege(privileges));
