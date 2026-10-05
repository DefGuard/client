import { describe, expect, it, vi } from 'vitest';
import { mfaConfigurableInstances } from './query';
import { ClientTrafficPolicy, type InstanceInfo, MfaMethod } from './types';

vi.mock('./api', () => ({ api: {} }));

const instanceOf = (id: number, overrides: Partial<InstanceInfo> = {}): InstanceInfo => ({
  id,
  name: `instance-${id}`,
  uuid: `uuid-${id}`,
  url: 'https://core.example',
  proxy_url: 'https://proxy.example',
  active: false,
  pubkey: 'pubkey',
  client_traffic_policy: ClientTrafficPolicy.None,
  enterprise_enabled: false,
  disable_tunnels: false,
  openid_display_name: null,
  openid_provider_kind: 'custom',
  mfa_configured_methods: [],
  mfa_capabilities: {
    setup_methods: [MfaMethod.Totp, MfaMethod.Email],
    authorize_methods: [MfaMethod.Totp],
  },
  smtp_configured: null,
  openid_available: null,
  ...overrides,
});

const idsOf = (instances: InstanceInfo[]) => instances.map((instance) => instance.id);

describe('mfaConfigurableInstances', () => {
  it('keeps an instance reporting nothing about SMTP or OpenID', () => {
    expect(idsOf(mfaConfigurableInstances([instanceOf(1)]))).toEqual([1]);
  });

  it('drops an instance that cannot configure from the client', () => {
    expect(mfaConfigurableInstances([instanceOf(1, { mfa_capabilities: null })])).toEqual(
      [],
    );
  });

  it('drops an instance whose only setup method is email without SMTP', () => {
    const instance = instanceOf(1, {
      smtp_configured: false,
      mfa_configured_methods: [MfaMethod.Totp],
      mfa_capabilities: {
        setup_methods: [MfaMethod.Email],
        authorize_methods: [MfaMethod.Totp],
      },
    });
    expect(mfaConfigurableInstances([instance])).toEqual([]);
  });

  it('drops an instance where nothing could verify the session', () => {
    const noSmtp = instanceOf(1, { smtp_configured: false });
    const onlyOpenId = instanceOf(2, {
      smtp_configured: false,
      openid_available: false,
      mfa_configured_methods: [MfaMethod.Oidc],
      mfa_capabilities: {
        setup_methods: [MfaMethod.Totp],
        authorize_methods: [MfaMethod.Oidc],
      },
    });
    expect(mfaConfigurableInstances([noSmtp, onlyOpenId])).toEqual([]);
  });

  it('keeps an instance without SMTP when a held factor verifies the session', () => {
    const instance = instanceOf(1, {
      smtp_configured: false,
      mfa_configured_methods: [MfaMethod.Totp],
    });
    expect(idsOf(mfaConfigurableInstances([instance]))).toEqual([1]);
  });
});
