CREATE TYPE header_pair AS (
    name TEXT,
    value BYTEA
);

CREATE TABLE idempotency (
    person_id uuid NOT NULL REFERENCES person(id),
    idempotency_key TEXT NOT NULL,
    response_status_code SMALLINT NOT NULL,
    response_headers header_pair[] NOT NULL,
    response_body BYTEA NOT NULL,
    created_at timestamptz NOT NULL,
    PRIMARY KEY(person_id, idempotency_key)
);
