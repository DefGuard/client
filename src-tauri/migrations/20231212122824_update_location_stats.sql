ALTER TABLE location_stats ADD COLUMN listen_port INTEGER NOT NULL DEFAULT 0;
ALTER TABLE location_stats ADD COLUMN persistent_keepalive_interval INTEGER NULL;
