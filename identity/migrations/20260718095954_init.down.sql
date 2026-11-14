-- Add down migration script here
DROP TABLE IF EXISTS "public"."t_user";
DROP TABLE IF EXISTS "public"."t_user_profile";
DROP TABLE IF EXISTS "public"."t_session";
DROP TABLE IF EXISTS "public"."t_passphrase";