-- Create links table
CREATE TABLE links (
    alias TEXT PRIMARY KEY,
    url   TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);