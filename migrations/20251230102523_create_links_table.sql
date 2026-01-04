-- Create links table
CREATE TABLE links (
    id BIGSERIAL PRIMARY KEY,
    alias TEXT UNIQUE,
    url TEXT NOT NULL,
    -- TODO: look into ON, and other datatypes
    hitcount BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
