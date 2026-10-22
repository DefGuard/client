import { describe, expect, it } from 'vitest';
import { MfaMethod } from '../../../shared/rust-api/types';
import { showsVerificationPicker } from './utils';

describe('showsVerificationPicker', () => {
  it('shows for more than one method', () => {
    expect(showsVerificationPicker([MfaMethod.Totp, MfaMethod.Email])).toBe(true);
  });

  it('shows for OpenID alone', () => {
    expect(showsVerificationPicker([MfaMethod.Oidc])).toBe(true);
  });

  it('skips for a single code method', () => {
    expect(showsVerificationPicker([MfaMethod.Totp])).toBe(false);
  });
});
