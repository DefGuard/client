import { describe, expect, it } from 'vitest';
import {
  ConnectionType,
  MfaMethod,
  type MfaMethodValue,
  type MfaStep,
} from '../../rust-api/types';
import { MfaFactorAction, type MfaSettingsLocation } from './types';
import { mfaSettingsStepsOf } from './utils';

const step = (...methods: [MfaMethodValue, boolean][]): MfaStep => ({
  methods: methods.map(([method, configured]) => ({ method, configured })),
});

const locationOf = (
  mfa_steps: MfaStep[],
  overrides: Partial<MfaSettingsLocation> = {},
): MfaSettingsLocation => ({
  connection_type: ConnectionType.Location,
  mfa_steps,
  mfa_step_plan: [],
  ...overrides,
});

const threeSteps = [
  step([MfaMethod.Totp, true], [MfaMethod.Email, false], [MfaMethod.Biometric, false]),
  step([MfaMethod.Oidc, true]),
  step([MfaMethod.Fido2, true], [MfaMethod.MobileApprove, false]),
];

const instance = {
  mfa_configured_methods: [MfaMethod.Totp, MfaMethod.Oidc, MfaMethod.Fido2],
};

describe('mfaSettingsStepsOf', () => {
  it('returns nothing for a location without steps', () => {
    expect(mfaSettingsStepsOf({ location: locationOf([]) })).toEqual([]);
  });

  it('returns nothing for a tunnel', () => {
    const location = locationOf(threeSteps, { connection_type: ConnectionType.Tunnel });
    expect(mfaSettingsStepsOf({ location, instance })).toEqual([]);
  });

  it('lists every step with its desktop-drivable factors', () => {
    const steps = mfaSettingsStepsOf({ location: locationOf(threeSteps), instance });
    expect(steps.map((entry) => entry.stepIndex)).toEqual([0, 1, 2]);
    expect(steps[0].factors.map((factor) => factor.method)).toEqual([
      MfaMethod.Totp,
      MfaMethod.Email,
    ]);
  });

  it('keeps only the requested steps', () => {
    const steps = mfaSettingsStepsOf({
      location: locationOf(threeSteps),
      instance,
      stepIndices: [1],
    });
    expect(steps).toHaveLength(1);
    expect(steps[0].stepIndex).toBe(1);
  });

  it('marks the resolved default per step', () => {
    const location = locationOf([step([MfaMethod.Totp, true], [MfaMethod.Email, true])], {
      mfa_step_plan: [MfaMethod.Email],
    });
    const [only] = mfaSettingsStepsOf({ location });
    expect(only.factors.map((factor) => factor.isDefault)).toEqual([false, true]);
  });

  it('leaves not-configured factors inert unless configurable', () => {
    const [first] = mfaSettingsStepsOf({ location: locationOf(threeSteps), instance });
    expect(first.factors).toEqual([
      {
        method: MfaMethod.Totp,
        configured: true,
        isDefault: true,
        action: MfaFactorAction.Pick,
      },
      {
        method: MfaMethod.Email,
        configured: false,
        isDefault: false,
        action: MfaFactorAction.None,
      },
    ]);
  });

  it('offers client-configurable factors for configuration', () => {
    const steps = mfaSettingsStepsOf({
      location: locationOf(threeSteps),
      instance,
      configurable: true,
    });
    expect(steps[0].factors[1].action).toBe(MfaFactorAction.Configure);
    // Mobile client factor is never configurable here.
    expect(steps[2].factors[1].action).toBe(MfaFactorAction.None);
  });

  it('never offers configuration on an instance that does not report its factors', () => {
    const steps = mfaSettingsStepsOf({
      location: locationOf(threeSteps),
      instance: { mfa_configured_methods: null },
      configurable: true,
    });
    expect(steps[0].factors[1].action).toBe(MfaFactorAction.None);
  });
});
