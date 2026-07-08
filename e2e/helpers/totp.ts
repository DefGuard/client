import { Secret, TOTP } from 'otpauth';

export const totpCode = (base32Secret: string): string =>
  new TOTP({
    secret: Secret.fromBase32(base32Secret.replace(/\s/g, '').toUpperCase()),
    digits: 6,
    period: 30,
  }).generate();
