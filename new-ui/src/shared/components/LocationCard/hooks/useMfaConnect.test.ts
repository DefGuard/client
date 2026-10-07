import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { type LocationInfo, MfaMethod } from '../../../rust-api/types';
import { useMfaConnect } from './useMfaConnect';

const mocks = vi.hoisted(() => ({
  error: vi.fn(),
  mfaBeginStep: vi.fn(),
  mfaFinishCode: vi.fn(),
  instances: [{ id: 42 }],
}));

vi.mock('@tanstack/react-query', () => ({
  useQuery: () => ({ data: mocks.instances }),
}));
vi.mock('@tauri-apps/plugin-log', () => ({ error: mocks.error }));
vi.mock('../../../rust-api/api', () => ({
  api: {
    mfaBeginStep: mocks.mfaBeginStep,
    mfaFinishCode: mocks.mfaFinishCode,
  },
}));
vi.mock('../../../rust-api/query', () => ({
  getInstancesQueryOptions: vi.fn(),
}));

const location = {
  id: 7,
  instance_id: 42,
  posture_check_required: false,
} as LocationInfo;

const renderCodeMfa = (setMfaToken: (token: string | null) => void) =>
  renderHook(() =>
    useMfaConnect(location, MfaMethod.Totp, {
      stepPlan: [MfaMethod.Totp],
      mfaToken: null,
      setMfaToken,
      onStepAdvanced: vi.fn(),
    }),
  );

describe('useMfaConnect', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.mfaBeginStep.mockResolvedValue({ token: 'session-token', stepAttemptId: null });
  });

  it('keeps the UI token after an invalid-code response', async () => {
    mocks.mfaFinishCode.mockRejectedValue(
      JSON.stringify({ type: 'mfa_rejected', message: 'invalid code' }),
    );
    const setMfaToken = vi.fn();
    const { result } = renderCodeMfa(setMfaToken);
    await waitFor(() => expect(result.current.token).toBe('session-token'));

    await act(async () => result.current.verifyCode('123456'));

    expect(result.current.verifyError).toBe('Invalid code');
    expect(result.current.token).toBe('session-token');
    expect(setMfaToken).not.toHaveBeenCalledWith(null);
  });

  it('clears the UI token after an attempt-limit response', async () => {
    mocks.mfaFinishCode.mockRejectedValue(
      JSON.stringify({ type: 'attempt_limit', message: 'Too many failed MFA attempts' }),
    );
    const setMfaToken = vi.fn();
    const { result } = renderCodeMfa(setMfaToken);
    await waitFor(() => expect(result.current.token).toBe('session-token'));

    await act(async () => result.current.verifyCode('123456'));

    expect(result.current.token).toBeNull();
    expect(setMfaToken).toHaveBeenCalledWith(null);
  });
});
