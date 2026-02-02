-- Create links table
CREATE TABLE links (
    id BIGSERIAL PRIMARY KEY,
    alias TEXT UNIQUE,
    url TEXT NOT NULL,
    hitcount BIGINT NOT NULL DEFAULT 0,
    last_access TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ
);


