-- Create links table
CREATE TABLE links (
    id BIGSERIAL PRIMARY KEY,
    alias TEXT UNIQUE,
    url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);