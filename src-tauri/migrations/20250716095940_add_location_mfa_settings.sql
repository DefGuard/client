-- add nullable column to `location` table
-- since SQLite does not support native enums we'll store them as integers
-- 1 - MFA disabled
-- 2 - internal MFA
-- 3 - external MFA
ALTER TABLE location ADD COLUMN location_mfa INTEGER NOT NULL DEFAULT 1;

-- populate new column based on value in `mfa_enabled` column
-- previously only internal MFA was available
UPDATE location
SET location_mfa = CASE
    WHEN mfa_enabled = true THEN 2
    ELSE 1
END;

-- drop the `mfa_enabled` column since it's no longer needed
ALTER TABLE location DROP COLUMN mfa_enabled;

-- remove `use_openid_for_mfa` setting
ALTER TABLE instance DROP COLUMN use_openid_for_mfa;
