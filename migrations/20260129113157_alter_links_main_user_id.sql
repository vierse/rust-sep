-- Add user_id column to links_main
ALTER TABLE links_main
ADD COLUMN user_id BIGINT;

-- Require user_id to point to a valid user
ALTER TABLE links_main
ADD CONSTRAINT links_main_user_id_fkey
FOREIGN KEY (user_id) REFERENCES users_main(id);

-- Index of links_main on user_id
CREATE INDEX links_main_user_id_idx ON links_main(user_id);