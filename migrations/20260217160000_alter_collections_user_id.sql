-- Add user_id column to collections
ALTER TABLE collections
ADD COLUMN user_id BIGINT;

-- Require user_id to point to a valid user
ALTER TABLE collections
ADD CONSTRAINT collections_user_id_fkey
FOREIGN KEY (user_id) REFERENCES users_main(id);

-- Index of collections on user_id
CREATE INDEX collections_user_id_idx ON collections(user_id);
