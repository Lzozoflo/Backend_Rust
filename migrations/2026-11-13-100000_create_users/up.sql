CREATE TABLE "users" (
    "id"            UUID         NOT NULL PRIMARY KEY,
    "username"      VARCHAR(32)  NOT NULL,
    "email"         VARCHAR(254) NOT NULL,
    "password_hash" TEXT         NOT NULL,
    "created_at"    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    "updated_at"    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    CONSTRAINT users_email_key UNIQUE ("email")
);

-- Unicité du username sans tenir compte de la casse.
-- (Un index fonctionnel : Diesel ne sait pas le générer, c'est pour ça qu'il est écrit à la main.)
CREATE UNIQUE INDEX users_username_lower_key ON "users" (lower("username"));
