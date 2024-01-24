-- update instance table
ALTER TABLE instance ADD COLUMN proxy_url TEXT NOT NULL;
ALTER TABLE instance ADD COLUMN username TEXT NOT NULL;

-- update location table
ALTER TABLE location ADD COLUMN mfa_enabled BOOLEAN NOT NULL;
ALTER TABLE location ADD COLUMN keepalive_interval INTEGER NOT NULL;
