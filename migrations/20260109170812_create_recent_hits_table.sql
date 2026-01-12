-- Add migration script here
CREATE TABLE recent_hits (
    -- id BIGINT PRIMARY KEY,
    accessed_at TIMESTAMP NOT NULL DEFAULT now(),
    link_id BIGINT NOT NULL REFERENCES links ON DELETE CASCADE
) PARTITION BY RANGE (accessed_at);
