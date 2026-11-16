UPDATE location SET mfa_steps = '[{"methods":[{"method":"totp","configured":true},{"method":"email","configured":true},{"method":"mobileapprove","configured":true}]}]'
WHERE mfa_steps = '[]' AND location_mfa_mode = 2;

UPDATE location SET mfa_steps = '[{"methods":[{"method":"oidc","configured":true}]}]'
WHERE mfa_steps = '[]' AND location_mfa_mode = 3;
