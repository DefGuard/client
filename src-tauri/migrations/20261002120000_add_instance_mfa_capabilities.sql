-- NULL marks a core that cannot configure MFA from the client, so no factor is offered there.
ALTER TABLE instance ADD COLUMN mfa_capabilities TEXT;
