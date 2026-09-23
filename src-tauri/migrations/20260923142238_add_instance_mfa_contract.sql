ALTER TABLE instance
ADD COLUMN mfa_contract INTEGER NOT NULL DEFAULT 0
    CHECK (mfa_contract IN (0, 1));
