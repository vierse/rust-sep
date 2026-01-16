
ALTER TABLE links ADD COLUMN IF NOT EXISTS last_accessed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_links_last_accessed_at ON links(last_accessed_at);

