-- Create main table to store links
CREATE TABLE links_main (
    id BIGSERIAL PRIMARY KEY,
    alias TEXT UNIQUE,
    url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    password_hash TEXT
);