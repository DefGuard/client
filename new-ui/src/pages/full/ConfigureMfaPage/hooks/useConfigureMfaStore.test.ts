import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../../../../shared/rust-api/api';
import {
  ClientTrafficPolicy,
  type InstanceInfo,
  MfaMethod,
} from '../../../../shared/rust-api/types';
import { ConfigureMfaStep } from '../types';
import {
  applyAuthorization,
  discardMfaConfiguration,
  startMfaConfiguration,
  useConfigureMfaStore,
} from './useConfigureMfaStore';

vi.mock('@tauri-apps/plugin-log', () => ({ error: vi.fn() }));
vi.mock('../../../../shared/rust-api/api', () => ({ api: {} }));

const sessionId = 'session-1';
const authorizeResult = { deadline_timestamp: 1_900_000_000, recovery_codes: [] };

describe('applyAuthorization', () => {
  beforeEach(() => {
    useConfigureMfaStore.getState().reset();
    useConfigureMfaStore.setState({
      sessionId,
      configuredMethods: [MfaMethod.Email],
      verificationMethods: [MfaMethod.Email],
    });
  });

  it('waits on the selection when the answer lands after Back', () => {
    useConfigureMfaStore.getState().selectMethods([MfaMethod.Fido2]);
    useConfigureMfaStore.getState().backFromVerification();
    applyAuthorization(sessionId, authorizeResult);

    const state = useConfigureMfaStore.getState();
    expect(state.authorized).toBe(true);
    expect(state.selectedMethods).toBeNull();
    expect(state.activeStep).toBe(ConfigureMfaStep.Configuration);
    expect(state.deadline).not.toBeNull();
  });

  it('sets up the picks confirmed after a late answer', () => {
    useConfigureMfaStore.getState().selectMethods([MfaMethod.Fido2]);
    useConfigureMfaStore.getState().backFromVerification();
    applyAuthorization(sessionId, authorizeResult);
    useConfigureMfaStore.getState().selectMethods([MfaMethod.Fido2]);

    const state = useConfigureMfaStore.getState();
    expect(state.activeStep).toBe(ConfigureMfaStep.Fido2);
    expect(state.deadline).not.toBeNull();
  });

  it('finishes when the pick confirmed after a late answer is empty', () => {
    useConfigureMfaStore.getState().selectMethods([MfaMethod.Fido2]);
    useConfigureMfaStore.getState().backFromVerification();
    applyAuthorization(sessionId, authorizeResult);
    useConfigureMfaStore.getState().selectMethods([]);

    const state = useConfigureMfaStore.getState();
    expect(state.activeStep).toBe(ConfigureMfaStep.Finish);
    expect(state.deadline).toBeNull();
  });

  it('keeps the deadline of an unauthorized session with an empty pick', () => {
    useConfigureMfaStore.setState({ deadline: '2030-01-01T00:00:00.000Z' });
    useConfigureMfaStore.getState().selectMethods([]);

    expect(useConfigureMfaStore.getState().deadline).toBe('2030-01-01T00:00:00.000Z');
  });

  it('drops an answer for another session', () => {
    useConfigureMfaStore.getState().selectMethods([MfaMethod.Fido2]);
    applyAuthorization('session-2', authorizeResult);

    expect(useConfigureMfaStore.getState().authorized).toBe(false);
  });
});

describe('startMfaConfiguration', () => {
  const instance: InstanceInfo = {
    id: 1,
    name: 'instance',
    uuid: 'instance-uuid',
    url: 'https://core.example',
    proxy_url: 'https://proxy.example',
    active: false,
    pubkey: 'pubkey',
    client_traffic_policy: ClientTrafficPolicy.None,
    enterprise_enabled: false,
    disable_tunnels: false,
    openid_display_name: null,
    mfa_configured_methods: [],
  };

  it('waits for the previous session to end before starting a new one', async () => {
    let endPrevious = () => {};
    const ended = new Promise<void>((resolve) => {
      endPrevious = resolve;
    });
    const mfaConfigStart = vi.fn().mockResolvedValue({
      session_id: 'session-2',
      available_methods: [MfaMethod.Totp],
      email_fallback: false,
      deadline_timestamp: 1_900_000_000,
    });
    Object.assign(api, { mfaConfigCancel: vi.fn(() => ended), mfaConfigStart });
    useConfigureMfaStore.setState({ sessionId });

    void discardMfaConfiguration();
    const started = startMfaConfiguration(instance);
    await Promise.resolve();
    expect(mfaConfigStart).not.toHaveBeenCalled();

    endPrevious();
    await started;
    expect(mfaConfigStart).toHaveBeenCalledOnce();
    expect(useConfigureMfaStore.getState().sessionId).toBe('session-2');
  });
});
