import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';
import { useConfigureVerifyFido2 } from './useConfigureVerifyFido2';

const mocks = vi.hoisted(() => ({
  error: vi.fn(),
  listen: vi.fn(),
  mfaConfigAbortAttempt: vi.fn(),
  mfaConfigAuthorizeFido2: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('@tauri-apps/plugin-log', () => ({ error: mocks.error }));
vi.mock('../../../../../shared/rust-api/api', () => ({
  api: {
    mfaConfigAbortAttempt: mocks.mfaConfigAbortAttempt,
    mfaConfigAuthorizeFido2: mocks.mfaConfigAuthorizeFido2,
  },
}));

const SESSION_ID = 'session-1';
const authorizeResult = { deadline_timestamp: 1_900_000_000, recovery_codes: [] };

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
};

const renderFido2 = () =>
  renderHook(() =>
    useConfigureVerifyFido2({ onSessionExpired: vi.fn(), autoStart: false }),
  );

describe('useConfigureVerifyFido2', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useConfigureMfaStore.getState().reset();
    useConfigureMfaStore.setState({ sessionId: SESSION_ID });
    mocks.listen.mockResolvedValue(vi.fn());
    mocks.mfaConfigAbortAttempt.mockResolvedValue(undefined);
    mocks.mfaConfigAuthorizeFido2.mockResolvedValue(authorizeResult);
  });

  it('does not start a ceremony when unmounted while listen is pending', async () => {
    const listening = deferred<() => void>();
    const unlisten = vi.fn();
    mocks.listen.mockReturnValue(listening.promise);
    const { result, unmount } = renderFido2();

    let verifying: Promise<void> | undefined;
    act(() => {
      verifying = result.current.verify(null);
    });
    unmount();
    listening.resolve(unlisten);
    await verifying;

    expect(mocks.mfaConfigAuthorizeFido2).not.toHaveBeenCalled();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('recovers when listen rejects', async () => {
    mocks.listen.mockRejectedValueOnce(new Error('no event bridge'));
    const { result } = renderFido2();

    await act(async () => {
      await result.current.verify(null);
    });
    expect(result.current.isVerifying).toBe(false);

    await act(async () => {
      await result.current.verify(null);
    });
    expect(mocks.mfaConfigAuthorizeFido2).toHaveBeenCalledTimes(1);
  });

  it('keeps an authorization that lands after unmount', async () => {
    const authorizing = deferred<typeof authorizeResult>();
    mocks.mfaConfigAuthorizeFido2.mockReturnValue(authorizing.promise);
    const { result, unmount } = renderFido2();

    let verifying: Promise<void> | undefined;
    await act(async () => {
      verifying = result.current.verify(null);
      await Promise.resolve();
    });
    unmount();
    authorizing.resolve(authorizeResult);
    await verifying;

    expect(useConfigureMfaStore.getState().authorized).toBe(true);
  });

  it('drops an authorization for a discarded session', async () => {
    const authorizing = deferred<typeof authorizeResult>();
    mocks.mfaConfigAuthorizeFido2.mockReturnValue(authorizing.promise);
    const { result } = renderFido2();

    let verifying: Promise<void> | undefined;
    await act(async () => {
      verifying = result.current.verify(null);
      await Promise.resolve();
    });
    act(() => {
      useConfigureMfaStore.getState().reset();
    });
    await act(async () => {
      authorizing.resolve(authorizeResult);
      await verifying;
    });

    expect(useConfigureMfaStore.getState().authorized).toBe(false);
  });
});
