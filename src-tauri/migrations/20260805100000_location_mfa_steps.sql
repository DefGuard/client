-- Multi-step MFA: replace the single `mfa_method` preference with a per-step
-- preference list (`user_mfa_preference`), and add `mfa_steps` describing the
-- methods available at each step. Both are JSON-encoded arrays stored as TEXT
-- since SQLite has no array type. Old `mfa_method` values are not migrated.
ALTER TABLE location ADD COLUMN user_mfa_preference TEXT;
ALTER TABLE location ADD COLUMN mfa_steps TEXT;
ALTER TABLE location DROP COLUMN mfa_method;
