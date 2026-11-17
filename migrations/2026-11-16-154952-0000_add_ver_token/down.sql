-- This file should undo anything in `up.sql`
ALTER TABLE "users" DROP COLUMN "token_version";
ALTER TABLE "users" DROP COLUMN "username";
ALTER TABLE "users" DROP COLUMN "email";
ALTER TABLE "users" ADD COLUMN "username" VARCHAR(32) NOT NULL;
ALTER TABLE "users" ADD COLUMN "email" VARCHAR(254) NOT NULL;

