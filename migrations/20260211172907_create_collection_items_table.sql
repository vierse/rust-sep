CREATE TABLE collections (
    id SERIAL PRIMARY KEY,
    alias TEXT UNIQUE NOT NULL,
    last_seen DATE NOT NULL DEFAULT CURRENT_DATE
);

CREATE TABLE collection_items (
    id SERIAL PRIMARY KEY,
    collection_id INT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    position INT NOT NULL,
    UNIQUE (collection_id, position)
);

CREATE TABLE collection_metrics (
    day DATE NOT NULL,
    collection_id BIGINT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    hits BIGINT NOT NULL,
    last_access TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (day, collection_id)
) PARTITION BY RANGE (day);
