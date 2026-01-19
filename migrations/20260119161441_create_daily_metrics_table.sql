-- Create metrics table with daily partitions
CREATE TABLE daily_metrics (
    day DATE NOT NULL,
    link_id BIGINT NOT NULL REFERENCES links_main(id) ON DELETE CASCADE,
    hits BIGINT NOT NULL,
    last_access TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (day, link_id)
) PARTITION BY RANGE (day);