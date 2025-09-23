ALTER TABLE instance ADD COLUMN use_openid_for_mfa BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE instance ADD COLUMN openid_display_name TEXT;
