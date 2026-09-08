import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { type LocationInfo, TauriEvent } from '../../../rust-api/types';
import { useMfaMobileConnect } from './useMfaMobileConnect';

const mocks = vi.hoisted(() => ({
  cancelMfa: vi.fn(),
  error: vi.fn(),
  listen: vi.fn(),
  mfaBeginStep: vi.fn(),
  mfaConnectMobileApprove: vi.fn(),
  setMfaToken: vi.fn(),
  instances: [{ id: 42, uuid: 'instance-uuid' }],
}));

vi.mock('@tanstack/react-query', () => ({
  useQuery: () => ({ data: mocks.instances }),
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('@tauri-apps/plugin-log', () => ({ error: mocks.error }));
vi.mock('../../../rust-api/api', () => ({
  api: {
    cancelMfa: mocks.cancelMfa,
    mfaBeginStep: mocks.mfaBeginStep,
    mfaConnectMobileApprove: mocks.mfaConnectMobileApprove,
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

type HookProps = {
  onConnected: () => void;
  onStepAdvanced: (nextStep: number) => void;
  mfaToken: string | null;
};

const renderMobileHook = (
  initialProps: HookProps = {
    onConnected: vi.fn(),
    onStepAdvanced: vi.fn(),
    mfaToken: null,
  },
) =>
  renderHook(
    ({ onConnected, onStepAdvanced, mfaToken }: HookProps) =>
      useMfaMobileConnect(location, {
        stepPlan: [],
        mfaToken,
        setMfaToken: mocks.setMfaToken,
        onStepAdvanced,
        onConnected,
      }),
    { initialProps },
  );

describe('useMfaMobileConnect', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.cancelMfa.mockResolvedValue(undefined);
    mocks.listen.mockResolvedValue(vi.fn());
  });

  it('does not re-park a rejected token after a callback rerender', async () => {
    mocks.mfaBeginStep.mockResolvedValue({
      challenge: 'challenge-1',
      token: 'token-1',
    });
    mocks.mfaConnectMobileApprove.mockRejectedValue(
      new Error('remote MFA wait superseded'),
    );

    const onConnected = vi.fn();
    const { result, rerender } = renderMobileHook({
      onConnected,
      onStepAdvanced: vi.fn(),
      mfaToken: null,
    });

    await act(async () => {
      await result.current.start();
    });
    await waitFor(() =>
      expect(result.current.connectionError).toBe(
        'Failed to start mobile approval. Please try again.',
      ),
    );
    expect(mocks.mfaConnectMobileApprove).toHaveBeenCalledTimes(1);

    await act(async () => {
      rerender({ onConnected, onStepAdvanced: vi.fn(), mfaToken: null });
      await Promise.resolve();
    });

    expect(mocks.mfaConnectMobileApprove).toHaveBeenCalledTimes(1);
    expect(mocks.mfaConnectMobileApprove).toHaveBeenCalledWith(42, 7, 'token-1');
  });

  it('delivers mobile events to the latest callbacks', async () => {
    mocks.mfaBeginStep.mockResolvedValue({
      challenge: 'challenge-1',
      token: 'token-1',
    });
    mocks.mfaConnectMobileApprove.mockResolvedValue('task-1');

    const firstConnected = vi.fn();
    const firstStepAdvanced = vi.fn();
    const latestConnected = vi.fn();
    const latestStepAdvanced = vi.fn();
    const { result, rerender } = renderMobileHook({
      onConnected: firstConnected,
      onStepAdvanced: firstStepAdvanced,
      mfaToken: null,
    });

    await act(async () => {
      await result.current.start();
    });
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledTimes(3));

    const completeListener = mocks.listen.mock.calls.find(
      ([event]) => event === TauriEvent.MfaMobileComplete,
    )?.[1] as (() => void) | undefined;
    const stepAdvancedListener = mocks.listen.mock.calls.find(
      ([event]) => event === TauriEvent.MfaMobileStepAdvanced,
    )?.[1] as ((event: { payload: { next_step: number } }) => void) | undefined;
    expect(completeListener).toBeDefined();
    expect(stepAdvancedListener).toBeDefined();

    await act(async () => {
      rerender({
        onConnected: latestConnected,
        onStepAdvanced: latestStepAdvanced,
        mfaToken: null,
      });
      await Promise.resolve();
    });
    await act(async () => {
      stepAdvancedListener?.({ payload: { next_step: 2 } });
      completeListener?.();
    });

    expect(firstStepAdvanced).not.toHaveBeenCalled();
    expect(firstConnected).not.toHaveBeenCalled();
    expect(latestStepAdvanced).toHaveBeenCalledWith(2);
    expect(latestConnected).toHaveBeenCalledTimes(1);
  });

  it('makes a mobile error terminal and clears the shared token', async () => {
    mocks.mfaBeginStep.mockResolvedValue({
      challenge: 'challenge-1',
      token: 'token-1',
    });
    mocks.mfaConnectMobileApprove.mockResolvedValue('task-1');

    const { result, rerender } = renderMobileHook();

    await act(async () => {
      await result.current.start();
    });
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledTimes(3));

    const errorListener = mocks.listen.mock.calls.find(
      ([event]) => event === TauriEvent.MfaMobileError,
    )?.[1] as ((event: { payload: { error: string } }) => void) | undefined;
    expect(errorListener).toBeDefined();

    await act(async () => {
      errorListener?.({ payload: { error: 'remote MFA wait superseded' } });
    });

    expect(result.current.connectionError).toBe('Connection error. Please try again.');
    expect(mocks.setMfaToken).toHaveBeenLastCalledWith(null);

    await act(async () => {
      rerender({
        onConnected: vi.fn(),
        onStepAdvanced: vi.fn(),
        mfaToken: null,
      });
      await Promise.resolve();
    });
    expect(mocks.mfaConnectMobileApprove).toHaveBeenCalledTimes(1);
  });

  it('uses a fresh token when explicitly retried', async () => {
    mocks.mfaBeginStep
      .mockResolvedValueOnce({ challenge: 'challenge-1', token: 'token-1' })
      .mockResolvedValueOnce({ challenge: 'challenge-2', token: 'token-2' });
    mocks.mfaConnectMobileApprove.mockRejectedValue(
      new Error('remote MFA wait superseded'),
    );

    const { result, rerender } = renderMobileHook({
      onConnected: vi.fn(),
      onStepAdvanced: vi.fn(),
      mfaToken: 'stale-token',
    });

    await act(async () => {
      await result.current.start();
    });
    await waitFor(() =>
      expect(result.current.connectionError).toBe(
        'Failed to start mobile approval. Please try again.',
      ),
    );
    expect(mocks.setMfaToken).toHaveBeenLastCalledWith(null);

    await act(async () => {
      rerender({
        onConnected: vi.fn(),
        onStepAdvanced: vi.fn(),
        mfaToken: null,
      });
    });
    await act(async () => {
      await result.current.start();
    });
    await waitFor(() => expect(mocks.mfaConnectMobileApprove).toHaveBeenCalledTimes(2));

    expect(mocks.mfaBeginStep.mock.calls[1]?.[4]).toBeNull();
    const parkedTokens = mocks.mfaConnectMobileApprove.mock.calls.map((call) => call[2]);
    expect(parkedTokens).toEqual(['token-1', 'token-2']);
  });
});
