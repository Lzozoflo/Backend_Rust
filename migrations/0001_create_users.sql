CREATE TABLE users (
    id            UUID        PRIMARY KEY,
    username      TEXT        NOT NULL,
    email         TEXT        NOT NULL,
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT users_email_key       UNIQUE (email),
    CONSTRAINT users_email_lowercase CHECK  (email = lower(email))
);

-- Username unique sans tenir compte de la casse.
CREATE UNIQUE INDEX users_username_key ON users (lower(username));

