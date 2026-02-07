-- Add last seen day to main links table
ALTER TABLE links_main
ADD COLUMN last_seen DATE NOT NULL DEFAULT CURRENT_DATE;