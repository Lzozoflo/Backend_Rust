-- Your SQL goes here
ALTER TABLE "users" DROP COLUMN "username";
ALTER TABLE "users" DROP COLUMN "email";
ALTER TABLE "users" ADD COLUMN "token_version" INT4 NOT NULL;
ALTER TABLE "users" ADD COLUMN "username" VARCHAR NOT NULL;
ALTER TABLE "users" ADD COLUMN "email" VARCHAR NOT NULL;

