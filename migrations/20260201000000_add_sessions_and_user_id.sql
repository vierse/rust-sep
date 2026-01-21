-- Add sessions table and link ownership
CREATE TABLE sessions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users ON DELETE CASCADE,
    session_token TEXT UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_session_token_idx ON sessions (session_token);

ALTER TABLE links
    ADD COLUMN user_id BIGINT REFERENCES users ON DELETE SET NULL;

CREATE INDEX links_user_id_idx ON links (user_id);
