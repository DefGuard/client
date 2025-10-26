-- 1 - disabled
-- 2 - pre-logon
-- 3 - always-on
ALTER TABLE location ADD COLUMN service_location_mode INTEGER NOT NULL DEFAULT 1;
