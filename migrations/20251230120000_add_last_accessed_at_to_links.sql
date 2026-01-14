-- Add last_accessed_at column to links table for tracking usage
ALTER TABLE links ADD COLUMN IF NOT EXISTS last_accessed_at TIMESTAMPTZ;

-- Create index for efficient queries on last_accessed_at
CREATE INDEX IF NOT EXISTS idx_links_last_accessed_at ON links(last_accessed_at);

