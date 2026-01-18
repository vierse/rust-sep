CREATE TABLE daily_hits (
    day DATE NOT NULL,
    link_id BIGINT NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    hits BIGINT NOT NULL,
    PRIMARY KEY (day, link_id)
) PARTITION BY RANGE (day);
