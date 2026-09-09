import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { type LocationInfo, TauriEvent } from '../../../rust-api/types';
import { useMfaFido2Connect } from './useMfaFido2Connect';

const mocks = vi.hoisted(() => ({
  cancelMfa: vi.fn(),
  error: vi.fn(),
  listen: vi.fn(),
  mfaFido2Pin: vi.fn(),
  setMfaToken: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('@tauri-apps/plugin-log', () => ({ error: mocks.error }));
vi.mock('../../../rust-api/api', () => ({
  api: {
    cancelMfa: mocks.cancelMfa,
    mfaFido2Pin: mocks.mfaFido2Pin,
  },
}));

const location = {
  id: 7,
  instance_id: 42,
  posture_check_required: false,
} as LocationInfo;

describe('useMfaFido2Connect', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.cancelMfa.mockResolvedValue(undefined);
    mocks.listen.mockResolvedValue(vi.fn());
    mocks.mfaFido2Pin.mockResolvedValue('task-1');
  });

  it('does not start a task after unmount during listener registration', async () => {
    const resolveListeners: Array<(unlisten: () => void) => void> = [];
    mocks.listen.mockImplementation(
      () =>
        new Promise<() => void>((resolve) => {
          resolveListeners.push(resolve);
        }),
    );

    const { result, unmount } = renderHook(() =>
      useMfaFido2Connect(location, {
        stepPlan: ['fido2'],
        mfaToken: 'preserved-token',
        setMfaToken: mocks.setMfaToken,
      }),
    );

    let verifyPromise!: Promise<void>;
    await act(async () => {
      verifyPromise = result.current.verifyPin('1234');
      await Promise.resolve();
    });
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledTimes(4));

    unmount();
    const unlisteners = [vi.fn(), vi.fn(), vi.fn(), vi.fn()];
    await act(async () => {
      resolveListeners[0]?.(unlisteners[0]);
      resolveListeners[1]?.(unlisteners[1]);
      resolveListeners[2]?.(unlisteners[2]);
      resolveListeners[3]?.(unlisteners[3]);
      await verifyPromise;
    });

    for (const unlisten of unlisteners) {
      expect(unlisten).toHaveBeenCalledTimes(1);
    }
    expect(mocks.mfaFido2Pin).not.toHaveBeenCalled();
    expect(mocks.setMfaToken).not.toHaveBeenCalledWith(null);
  });

  it('does not attach a late task after an advanced event', async () => {
    let resolveTask!: (taskId: string) => void;
    mocks.mfaFido2Pin.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolveTask = resolve;
        }),
    );

    const onConnected = vi.fn();
    const onStepAdvanced = vi.fn();
    const { result, unmount } = renderHook(() =>
      useMfaFido2Connect(location, {
        stepPlan: ['fido2', 'totp'],
        mfaToken: null,
        setMfaToken: mocks.setMfaToken,
        onConnected,
        onStepAdvanced,
      }),
    );

    let verifyPromise!: Promise<void>;
    await act(async () => {
      verifyPromise = result.current.verifyPin('1234');
      await Promise.resolve();
    });
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledTimes(4));

    const stepAdvancedListener = mocks.listen.mock.calls.find(
      ([event]) => event === TauriEvent.MfaFido2StepAdvanced,
    )?.[1] as
      | ((event: { payload: { next_step: number; token: string } }) => void)
      | undefined;
    expect(stepAdvancedListener).toBeDefined();

    await act(async () => {
      stepAdvancedListener?.({ payload: { next_step: 1, token: 'fido2-token' } });
    });
    await act(async () => {
      resolveTask('late-task');
      await verifyPromise;
    });

    unmount();

    expect(mocks.cancelMfa).toHaveBeenCalledWith('late-task');
    expect(mocks.setMfaToken).toHaveBeenLastCalledWith('fido2-token');
    expect(onStepAdvanced).toHaveBeenCalledWith(1);
    expect(onConnected).not.toHaveBeenCalled();
  });

  it('stores the token and advances on a non-final FIDO2 step', async () => {
    const onConnected = vi.fn();
    const onStepAdvanced = vi.fn();
    const { result } = renderHook(() =>
      useMfaFido2Connect(location, {
        stepPlan: ['fido2', 'totp'],
        mfaToken: null,
        setMfaToken: mocks.setMfaToken,
        onConnected,
        onStepAdvanced,
      }),
    );

    await act(async () => {
      await result.current.verifyPin('1234');
    });
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledTimes(4));

    const stepAdvancedListener = mocks.listen.mock.calls.find(
      ([event]) => event === TauriEvent.MfaFido2StepAdvanced,
    )?.[1] as
      | ((event: { payload: { next_step: number; token: string } }) => void)
      | undefined;
    expect(stepAdvancedListener).toBeDefined();

    await act(async () => {
      stepAdvancedListener?.({ payload: { next_step: 1, token: 'fido2-token' } });
    });

    expect(mocks.setMfaToken).toHaveBeenLastCalledWith('fido2-token');
    expect(onStepAdvanced).toHaveBeenCalledWith(1);
    expect(onConnected).not.toHaveBeenCalled();
    expect(result.current.isVerifying).toBe(false);
  });
});
