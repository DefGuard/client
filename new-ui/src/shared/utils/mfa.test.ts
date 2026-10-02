import { describe, expect, it } from 'vitest';
import {
  ConnectionType,
  type MfaCapabilities,
  MfaMethod,
  type MfaMethodValue,
  type MfaStep,
} from '../rust-api/types';
import {
  ConnectionAbility,
  canSetUpMfaMethod,
  connectionAbilityOf,
  hasMfaMethodChoice,
  setupMethodsOf,
} from './mfa';

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

const capabilitiesOf = (...setup_methods: MfaMethodValue[]): MfaCapabilities => ({
  setup_methods,
  authorize_methods: [],
});

describe('setupMethodsOf', () => {
  it('is empty for an instance that cannot configure from the client', () => {
    expect(setupMethodsOf({ mfa_capabilities: null })).toEqual([]);
    expect(setupMethodsOf(undefined)).toEqual([]);
  });

  it('keeps the client order and drops what the client cannot set up', () => {
    const instance = {
      mfa_capabilities: capabilitiesOf(
        MfaMethod.Fido2,
        MfaMethod.MobileApprove,
        MfaMethod.Totp,
      ),
    };
    expect(setupMethodsOf(instance)).toEqual([MfaMethod.Totp, MfaMethod.Fido2]);
  });
});

describe('canSetUpMfaMethod', () => {
  it('needs both the client and the instance', () => {
    const instance = { mfa_capabilities: capabilitiesOf(MfaMethod.Totp, MfaMethod.Oidc) };
    expect(canSetUpMfaMethod(MfaMethod.Totp, instance)).toBe(true);
    expect(canSetUpMfaMethod(MfaMethod.Email, instance)).toBe(false);
    expect(canSetUpMfaMethod(MfaMethod.Oidc, instance)).toBe(false);
  });
});

describe('connectionAbilityOf', () => {
  const unconfiguredStep: MfaStep = {
    methods: [
      { method: MfaMethod.Email, configured: false },
      { method: MfaMethod.Biometric, configured: false },
    ],
  };
  const location = locationOf([unconfiguredStep]);

  it('is configurable when the instance can set up a blocking factor', () => {
    const instance = {
      mfa_configured_methods: [],
      mfa_capabilities: capabilitiesOf(MfaMethod.Email),
    };
    expect(connectionAbilityOf(location, instance)).toBe(ConnectionAbility.Configurable);
  });

  it('is unavailable when the instance cannot set up any blocking factor', () => {
    const instance = {
      mfa_configured_methods: [],
      mfa_capabilities: capabilitiesOf(MfaMethod.Totp),
    };
    expect(connectionAbilityOf(location, instance)).toBe(ConnectionAbility.Unavailable);
  });

  it('is unavailable when the instance cannot configure from the client', () => {
    const instance = { mfa_configured_methods: [], mfa_capabilities: null };
    expect(connectionAbilityOf(location, instance)).toBe(ConnectionAbility.Unavailable);
  });
});
