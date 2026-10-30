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

const renderCodeMfa = (
  setMfaToken: (token: string | null) => void,
  onFailure?: (err: unknown) => void,
) =>
  renderHook(() =>
    useMfaConnect(location, MfaMethod.Totp, {
      stepPlan: [MfaMethod.Totp],
      mfaToken: null,
      setMfaToken,
      onStepAdvanced: vi.fn(),
      onFailure,
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

  describe('with onFailure', () => {
    it('keeps an invalid code on the input', async () => {
      mocks.mfaFinishCode.mockRejectedValue(
        JSON.stringify({ type: 'mfa_rejected', message: 'invalid code' }),
      );
      const onFailure = vi.fn();
      const { result } = renderCodeMfa(vi.fn(), onFailure);
      await waitFor(() => expect(result.current.token).toBe('session-token'));

      await act(async () => result.current.verifyCode('123456'));

      expect(result.current.verifyError).toBe('Invalid code');
      expect(onFailure).not.toHaveBeenCalled();
    });

    it.each([
      ['attempt limit', JSON.stringify({ type: 'attempt_limit', message: 'Too many' })],
      ['session expired', JSON.stringify({ type: 'other', message: 'invalid token' })],
      ['VPN failure', 'VPN connection failed: interface error'],
    ])('reports %s instead of showing it on the input', async (_, err) => {
      mocks.mfaFinishCode.mockRejectedValue(err);
      const onFailure = vi.fn();
      const { result } = renderCodeMfa(vi.fn(), onFailure);
      await waitFor(() => expect(result.current.token).toBe('session-token'));

      await act(async () => result.current.verifyCode('123456'));

      expect(onFailure).toHaveBeenCalledWith(err);
      expect(result.current.verifyError).toBeNull();
    });

    it('reports a failed start instead of exposing startError', async () => {
      const err = JSON.stringify({ type: 'network_error', message: 'unreachable' });
      mocks.mfaBeginStep.mockRejectedValue(err);
      const onFailure = vi.fn();
      const { result } = renderCodeMfa(vi.fn(), onFailure);

      await waitFor(() => expect(onFailure).toHaveBeenCalledWith(err));
      expect(result.current.startError).toBeNull();
    });
  });
});
