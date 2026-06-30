-- Add migration script here
ALTER TABLE photos
    ADD COLUMN claimed_at TIMESTAMPTZ,
    ADD COLUMN attempts integer NOT NULL DEFAULT 0;
