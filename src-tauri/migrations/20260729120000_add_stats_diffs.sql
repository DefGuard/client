ALTER TABLE location_stats ADD COLUMN upload_diff INTEGER NOT NULL DEFAULT 0;
ALTER TABLE location_stats ADD COLUMN download_diff INTEGER NOT NULL DEFAULT 0;

ALTER TABLE tunnel_stats ADD COLUMN upload_diff BIGINT NOT NULL DEFAULT 0;
ALTER TABLE tunnel_stats ADD COLUMN download_diff BIGINT NOT NULL DEFAULT 0;

CREATE INDEX idx_location_stats_location_collected
    ON location_stats (location_id, collected_at);

CREATE INDEX idx_tunnel_stats_tunnel_collected
    ON tunnel_stats (tunnel_id, collected_at);
