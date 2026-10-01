import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MfaMethod } from '../../../../shared/rust-api/types';
import { ConfigureMfaStep } from '../types';
import { applyAuthorization, useConfigureMfaStore } from './useConfigureMfaStore';

vi.mock('@tauri-apps/plugin-log', () => ({ error: vi.fn() }));
vi.mock('../../../../shared/rust-api/api', () => ({ api: {} }));

const SESSION_ID = 'session-1';
const authorizeResult = { deadline_timestamp: 1_900_000_000, recovery_codes: [] };

describe('applyAuthorization', () => {
  beforeEach(() => {
    useConfigureMfaStore.getState().reset();
    useConfigureMfaStore.setState({
      sessionId: SESSION_ID,
      configuredMethods: [MfaMethod.Email],
      verificationMethods: [MfaMethod.Email],
    });
  });

  it('sets up the picks when the answer lands after Back to the selection', () => {
    useConfigureMfaStore.getState().selectMethods([MfaMethod.Fido2]);
    useConfigureMfaStore.getState().backFromVerification();
    applyAuthorization(SESSION_ID, authorizeResult);

    const state = useConfigureMfaStore.getState();
    expect(state.authorized).toBe(true);
    expect(state.selectedMethods).toEqual([MfaMethod.Fido2]);
    expect(state.activeStep).toBe(ConfigureMfaStep.Fido2);
    expect(state.deadline).not.toBeNull();
  });

  it('finishes when the parked selection is empty', () => {
    useConfigureMfaStore.getState().selectMethods([]);
    useConfigureMfaStore.getState().backFromVerification();
    applyAuthorization(SESSION_ID, authorizeResult);

    const state = useConfigureMfaStore.getState();
    expect(state.activeStep).toBe(ConfigureMfaStep.Finish);
    expect(state.deadline).toBeNull();
  });

  it('drops an answer for another session', () => {
    useConfigureMfaStore.getState().selectMethods([MfaMethod.Fido2]);
    applyAuthorization('session-2', authorizeResult);

    expect(useConfigureMfaStore.getState().authorized).toBe(false);
  });
});
