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
  canAuthorizeMfaConfig,
  canSetUpMfaMethod,
  connectionAbilityOf,
  hasMfaMethodChoice,
  isMfaMethodAvailable,
  resolveMfaStepPlan,
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

  it('does not count a factor the instance cannot run', () => {
    const location = locationOf([step(MfaMethod.Email, MfaMethod.Totp)]);
    const noSmtp = { smtp_configured: false, openid_available: null };
    expect(hasMfaMethodChoice(location, noSmtp)).toBe(false);
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

const capabilitiesOf = (
  setup_methods: MfaMethodValue[],
  authorize_methods: MfaMethodValue[] = [],
): MfaCapabilities => ({
  setup_methods,
  authorize_methods,
});

const unreported = { smtp_configured: null, openid_available: null };

describe('isMfaMethodAvailable', () => {
  it('reads an unreported flag as available', () => {
    expect(isMfaMethodAvailable(MfaMethod.Email, unreported)).toBe(true);
    expect(isMfaMethodAvailable(MfaMethod.Oidc, unreported)).toBe(true);
    expect(isMfaMethodAvailable(MfaMethod.Email, undefined)).toBe(true);
  });

  it('follows the flag of the factor it gates', () => {
    const instance = { smtp_configured: false, openid_available: false };
    expect(isMfaMethodAvailable(MfaMethod.Email, instance)).toBe(false);
    expect(isMfaMethodAvailable(MfaMethod.Oidc, instance)).toBe(false);
    expect(isMfaMethodAvailable(MfaMethod.Totp, instance)).toBe(true);
  });
});

describe('setupMethodsOf', () => {
  it('is empty for an instance that cannot configure from the client', () => {
    expect(setupMethodsOf({ ...unreported, mfa_capabilities: null })).toEqual([]);
    expect(setupMethodsOf(undefined)).toEqual([]);
  });

  it('keeps the client order and drops what the client cannot set up', () => {
    const instance = {
      ...unreported,
      mfa_capabilities: capabilitiesOf([
        MfaMethod.Fido2,
        MfaMethod.MobileApprove,
        MfaMethod.Totp,
      ]),
    };
    expect(setupMethodsOf(instance)).toEqual([MfaMethod.Totp, MfaMethod.Fido2]);
  });

  it('drops email on an instance without SMTP', () => {
    const instance = {
      ...unreported,
      smtp_configured: false,
      mfa_capabilities: capabilitiesOf([MfaMethod.Totp, MfaMethod.Email]),
    };
    expect(setupMethodsOf(instance)).toEqual([MfaMethod.Totp]);
  });
});

describe('canSetUpMfaMethod', () => {
  it('needs both the client and the instance', () => {
    const instance = {
      ...unreported,
      mfa_capabilities: capabilitiesOf([MfaMethod.Totp, MfaMethod.Oidc]),
    };
    expect(canSetUpMfaMethod(MfaMethod.Totp, instance)).toBe(true);
    expect(canSetUpMfaMethod(MfaMethod.Email, instance)).toBe(false);
    expect(canSetUpMfaMethod(MfaMethod.Oidc, instance)).toBe(false);
  });
});

describe('canAuthorizeMfaConfig', () => {
  it('falls back to email when the account holds no authorizer', () => {
    const instance = {
      ...unreported,
      mfa_configured_methods: [],
      mfa_capabilities: capabilitiesOf([], [MfaMethod.Totp]),
    };
    expect(canAuthorizeMfaConfig(instance)).toBe(true);
    expect(canAuthorizeMfaConfig({ ...instance, smtp_configured: false })).toBe(false);
  });

  it('accepts a held factor the instance authorizes with', () => {
    const instance = {
      ...unreported,
      smtp_configured: false,
      mfa_configured_methods: [MfaMethod.Totp],
      mfa_capabilities: capabilitiesOf([], [MfaMethod.Totp]),
    };
    expect(canAuthorizeMfaConfig(instance)).toBe(true);
  });

  it('ignores a held factor the instance cannot run', () => {
    const instance = {
      smtp_configured: false,
      openid_available: false,
      mfa_configured_methods: [MfaMethod.Oidc],
      mfa_capabilities: capabilitiesOf([], [MfaMethod.Oidc]),
    };
    expect(canAuthorizeMfaConfig(instance)).toBe(false);
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
      ...unreported,
      mfa_configured_methods: [],
      mfa_capabilities: capabilitiesOf([MfaMethod.Email]),
    };
    expect(connectionAbilityOf(location, instance)).toBe(ConnectionAbility.Configurable);
  });

  it('is unavailable when the instance cannot set up any blocking factor', () => {
    const instance = {
      ...unreported,
      mfa_configured_methods: [],
      mfa_capabilities: capabilitiesOf([MfaMethod.Totp]),
    };
    expect(connectionAbilityOf(location, instance)).toBe(ConnectionAbility.Unavailable);
  });

  it('is unavailable when the instance cannot configure from the client', () => {
    const instance = {
      ...unreported,
      mfa_configured_methods: [],
      mfa_capabilities: null,
    };
    expect(connectionAbilityOf(location, instance)).toBe(ConnectionAbility.Unavailable);
  });

  it('is unavailable when the only factor is email on an instance without SMTP', () => {
    const instance = {
      ...unreported,
      smtp_configured: false,
      mfa_configured_methods: [MfaMethod.Email],
      mfa_capabilities: capabilitiesOf([MfaMethod.Email], [MfaMethod.Email]),
    };
    expect(connectionAbilityOf(locationOf([step(MfaMethod.Email)]), instance)).toBe(
      ConnectionAbility.Unavailable,
    );
  });

  it('is unavailable when the only factor is OpenID on an instance without a provider', () => {
    const instance = {
      ...unreported,
      openid_available: false,
      mfa_configured_methods: [MfaMethod.Oidc],
      mfa_capabilities: capabilitiesOf([MfaMethod.Totp], [MfaMethod.Totp]),
    };
    expect(connectionAbilityOf(locationOf([step(MfaMethod.Oidc)]), instance)).toBe(
      ConnectionAbility.Unavailable,
    );
  });

  it('is unavailable when nothing could authorize the setup', () => {
    const instance = {
      ...unreported,
      smtp_configured: false,
      mfa_configured_methods: [],
      mfa_capabilities: capabilitiesOf([MfaMethod.Totp], [MfaMethod.Totp]),
    };
    expect(connectionAbilityOf(locationOf([step(MfaMethod.Totp)]), instance)).toBe(
      ConnectionAbility.Unavailable,
    );
  });

  it('is configurable when a held factor can authorize the setup', () => {
    const instance = {
      ...unreported,
      smtp_configured: false,
      mfa_configured_methods: [MfaMethod.Totp],
      mfa_capabilities: capabilitiesOf([MfaMethod.Fido2], [MfaMethod.Totp]),
    };
    expect(connectionAbilityOf(locationOf([step(MfaMethod.Fido2)]), instance)).toBe(
      ConnectionAbility.Configurable,
    );
  });
});

describe('resolveMfaStepPlan', () => {
  it('skips a held factor the instance cannot run', () => {
    const location = {
      ...locationOf([step(MfaMethod.Email, MfaMethod.Totp)]),
      mfa_step_plan: [],
    };
    const instance = {
      ...unreported,
      smtp_configured: false,
      mfa_configured_methods: [MfaMethod.Email, MfaMethod.Totp],
    };
    expect(resolveMfaStepPlan(location)).toEqual([MfaMethod.Email]);
    expect(resolveMfaStepPlan(location, [], instance)).toEqual([MfaMethod.Totp]);
  });
});
