-- Add nullable expires_at column to links table
ALTER TABLE links
    ADD COLUMN expires_at TIMESTAMPTZ;
