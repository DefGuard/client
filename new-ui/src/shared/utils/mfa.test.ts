import { describe, expect, it } from 'vitest';
import {
  ConnectionType,
  MfaMethod,
  type MfaMethodValue,
  type MfaStep,
} from '../rust-api/types';
import { hasMfaMethodChoice } from './mfa';

const step = (...methods: MfaMethodValue[]): MfaStep => ({
  methods: methods.map((method) => ({ method, configured: true })),
});

const locationOf = (
  mfa_steps: MfaStep[],
  connection_type: ConnectionType = ConnectionType.Location,
) => ({ connection_type, mfa_steps });

describe('hasMfaMethodChoice', () => {
  it('is false for a single step with a single factor', () => {
    expect(hasMfaMethodChoice(locationOf([step(MfaMethod.Email)]))).toBe(false);
  });

  it('is true when a step offers two factors', () => {
    expect(hasMfaMethodChoice(locationOf([step(MfaMethod.Email, MfaMethod.Totp)]))).toBe(
      true,
    );
  });

  it('is false for several single-factor steps', () => {
    expect(
      hasMfaMethodChoice(locationOf([step(MfaMethod.Email), step(MfaMethod.Oidc)])),
    ).toBe(false);
  });

  it('does not count biometric as a desktop choice', () => {
    expect(
      hasMfaMethodChoice(locationOf([step(MfaMethod.Totp, MfaMethod.Biometric)])),
    ).toBe(false);
  });

  it('is false for a tunnel', () => {
    expect(
      hasMfaMethodChoice(
        locationOf([step(MfaMethod.Email, MfaMethod.Totp)], ConnectionType.Tunnel),
      ),
    ).toBe(false);
  });
});
