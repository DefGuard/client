-- add client_traffic_policy column to `instance` table
-- since SQLite does not support native enums we'll store them as integers
-- 0 - None
-- 1 - Disable all traffic
-- 2 - Force all traffic
ALTER TABLE instance ADD COLUMN client_traffic_policy INTEGER NOT NULL DEFAULT 0;

-- populate new column based on value in `disable_all_traffic` column
UPDATE instance
SET client_traffic_policy = CASE
    WHEN disable_all_traffic = true THEN 1
    ELSE 0
END;

-- drop the `disable_all_traffic` column since it's no longer needed
ALTER TABLE instance DROP COLUMN disable_all_traffic;
