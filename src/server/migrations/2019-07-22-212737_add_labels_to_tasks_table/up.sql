ALTER TABLE tasks ADD COLUMN labels Text[] NOT NULL DEFAULT '{}';

CREATE FUNCTION automaat_validate_label(txt Text[]) RETURNS boolean AS $$
    SELECT bool_and (str ~ '^[a-z0-9_]+$') FROM unnest(txt) s(str);
$$ IMMUTABLE STRICT LANGUAGE SQL;

ALTER TABLE tasks ADD CONSTRAINT label_syntax CHECK (automaat_validate_label(labels));
