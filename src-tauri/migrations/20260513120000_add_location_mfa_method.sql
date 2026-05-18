-- 0 - TOTP
-- 2 - OIDC
-- NULL - unset (used for Disabled MFA mode locations)
ALTER TABLE location ADD COLUMN mfa_method INTEGER;
UPDATE location SET mfa_method = 0 WHERE location_mfa_mode = 2;
UPDATE location SET mfa_method = 2 WHERE location_mfa_mode = 3;
