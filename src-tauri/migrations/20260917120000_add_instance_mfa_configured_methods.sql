-- NULL marks an instance whose proxy never reported MFA state: it predates the API and cannot
-- configure factors from the client at all. '[]' is a proxy that reported no configured factors.
ALTER TABLE instance ADD COLUMN mfa_configured_methods TEXT;
