-- Create links table
CREATE TABLE links (
    id BIGSERIAL PRIMARY KEY,
    alias TEXT UNIQUE,
    url TEXT NOT NULL,
    -- TODO: look into ON, and other datatypes
    hitcount BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- look into wether this should be turned into a VecDeuqe or sth
CREATE TABLE recent_hits (
    -- id BIGINT PRIMARY KEY,
    accessed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    link_id BIGINT NOT NULL REFERENCES links ON DELETE CASCADE
);
