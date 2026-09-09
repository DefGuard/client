import { act, renderHook, waitFor } from '@testing-library/react';
import { StrictMode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMfaOidcConnect } from './useMfaOidcConnect';

const mocks = vi.hoisted(() => ({
  cancelMfa: vi.fn(),
  error: vi.fn(),
  goToStep: vi.fn(),
  listen: vi.fn(),
  mfaBeginStep: vi.fn(),
  mfaPollOpenId: vi.fn(),
  openLink: vi.fn(),
  setMfaToken: vi.fn(),
  setPostureError: vi.fn(),
  setView: vi.fn(),
  useLocationCardContext: vi.fn(),
}));

vi.mock('@tanstack/react-query', () => ({
  useQuery: () => ({ data: [{ id: 42, proxy_url: 'https://proxy.example/' }] }),
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('@tauri-apps/plugin-log', () => ({ error: mocks.error }));
vi.mock('../../../rust-api/api', () => ({
  api: {
    cancelMfa: mocks.cancelMfa,
    mfaBeginStep: mocks.mfaBeginStep,
    mfaPollOpenId: mocks.mfaPollOpenId,
    openLink: mocks.openLink,
  },
}));
vi.mock('../../../rust-api/query', () => ({
  getInstancesQueryOptions: vi.fn(),
}));
vi.mock('../context/context', () => ({
  useLocationCardContext: mocks.useLocationCardContext,
}));

describe('useMfaOidcConnect', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.useLocationCardContext.mockReturnValue({
      goToStep: mocks.goToStep,
      location: { id: 7, instance_id: 42 },
      mfaToken: null,
      setMfaToken: mocks.setMfaToken,
      setPostureError: mocks.setPostureError,
      setView: mocks.setView,
      stepPlan: ['oidc'],
    });
    mocks.cancelMfa.mockResolvedValue(undefined);
    mocks.mfaBeginStep.mockResolvedValue({
      challenge: null,
      stepAttemptId: 'attempt-1',
      token: 'mfa-token',
    });
    mocks.mfaPollOpenId.mockResolvedValue('task-1');
    mocks.openLink.mockResolvedValue(undefined);
    mocks.listen.mockResolvedValue(vi.fn());
  });

  it('starts OIDC once when StrictMode replays the auto-start effect', async () => {
    renderHook(() => useMfaOidcConnect(true), { wrapper: StrictMode });

    await waitFor(() => {
      expect(mocks.mfaBeginStep).toHaveBeenCalledTimes(1);
      expect(mocks.openLink).toHaveBeenCalledTimes(1);
      expect(mocks.mfaPollOpenId).toHaveBeenCalledTimes(1);
    });
  });

  it('cancels the deferred auto-start on unmount', async () => {
    const { unmount } = renderHook(() => useMfaOidcConnect(true));

    unmount();
    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    });

    expect(mocks.mfaBeginStep).not.toHaveBeenCalled();
  });

  it('opens OIDC with the raw token and step attempt ID as separate parameters', async () => {
    const { result } = renderHook(() => useMfaOidcConnect());

    await act(async () => {
      await result.current.start();
    });

    const link = mocks.openLink.mock.calls[0]?.[0] as string;
    const url = new URL(link);

    expect(url.pathname).toBe('/openid/mfa');
    expect(url.searchParams.get('token')).toBe('mfa-token');
    expect(url.searchParams.get('step_attempt_id')).toBe('attempt-1');
  });

  it('keeps the legacy token-only URL when no step attempt ID is returned', async () => {
    mocks.mfaBeginStep.mockResolvedValue({
      challenge: null,
      stepAttemptId: null,
      token: 'mfa-token',
    });

    const { result } = renderHook(() => useMfaOidcConnect());

    await act(async () => {
      await result.current.start();
    });

    const url = new URL(mocks.openLink.mock.calls[0]?.[0] as string);

    expect(url.searchParams.get('token')).toBe('mfa-token');
    expect(url.searchParams.has('step_attempt_id')).toBe(false);
  });

  it('does not continue starting MFA after unmount during session creation', async () => {
    type Session = {
      challenge: null;
      stepAttemptId: string;
      token: string;
    };
    let resolveStart!: (session: Session) => void;
    mocks.mfaBeginStep.mockImplementation(
      () =>
        new Promise<Session>((resolve) => {
          resolveStart = resolve;
        }),
    );
    const { result, unmount } = renderHook(() => useMfaOidcConnect());
    let startPromise: Promise<void> | undefined;

    await act(async () => {
      startPromise = result.current.start();
      await Promise.resolve();
    });
    await waitFor(() => expect(mocks.mfaBeginStep).toHaveBeenCalledTimes(1));

    unmount();
    await act(async () => {
      resolveStart({ challenge: null, stepAttemptId: 'attempt-1', token: 'mfa-token' });
      await startPromise;
    });

    expect(mocks.setMfaToken).not.toHaveBeenCalled();
    expect(mocks.openLink).not.toHaveBeenCalled();
    expect(mocks.mfaPollOpenId).not.toHaveBeenCalled();
  });

  it('cleans a listener registered after unmount', async () => {
    const unlisten = vi.fn();
    let resolveListener!: (unlisten: () => void) => void;
    mocks.listen.mockResolvedValue(unlisten);
    mocks.listen.mockImplementationOnce(
      () =>
        new Promise<() => void>((resolve) => {
          resolveListener = resolve;
        }),
    );
    const { result, unmount } = renderHook(() => useMfaOidcConnect());
    let startPromise: Promise<void> | undefined;

    await act(async () => {
      startPromise = result.current.start();
      await Promise.resolve();
    });
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledTimes(1));

    unmount();
    await act(async () => {
      resolveListener(unlisten);
      await startPromise;
    });

    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(mocks.listen).toHaveBeenCalledTimes(1);
  });

  it('cleans partial listener registration on unmount', async () => {
    const firstUnlisten = vi.fn();
    const secondUnlisten = vi.fn();
    let resolveSecond!: (unlisten: () => void) => void;
    mocks.listen.mockImplementationOnce(() => Promise.resolve(firstUnlisten));
    mocks.listen.mockImplementationOnce(
      () =>
        new Promise<() => void>((resolve) => {
          resolveSecond = resolve;
        }),
    );
    const { result, unmount } = renderHook(() => useMfaOidcConnect());
    let startPromise: Promise<void> | undefined;

    await act(async () => {
      startPromise = result.current.start();
      await Promise.resolve();
    });
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledTimes(2));

    unmount();
    expect(firstUnlisten).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveSecond(secondUnlisten);
      await startPromise;
    });

    expect(secondUnlisten).toHaveBeenCalledTimes(1);
    expect(mocks.listen).toHaveBeenCalledTimes(2);
    expect(mocks.cancelMfa).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('task-1');
  });

  it('does not let stale listener cleanup cancel a newer retry', async () => {
    const firstAUnlisten = vi.fn();
    const staleAUnlisten = vi.fn();
    const bUnlisten = vi.fn();
    let resolveSecondA!: (unlisten: () => void) => void;
    mocks.listen.mockResolvedValue(bUnlisten);
    mocks.listen.mockImplementationOnce(() => Promise.resolve(firstAUnlisten));
    mocks.listen.mockImplementationOnce(
      () =>
        new Promise<() => void>((resolve) => {
          resolveSecondA = resolve;
        }),
    );
    mocks.mfaPollOpenId.mockResolvedValueOnce('task-a').mockResolvedValueOnce('task-b');
    const { result, unmount } = renderHook(() => useMfaOidcConnect());
    let startA: Promise<void> | undefined;

    await act(async () => {
      startA = result.current.start();
      await Promise.resolve();
    });
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledTimes(2));

    await act(async () => {
      await result.current.start();
    });
    expect(mocks.cancelMfa).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('task-a');
    expect(mocks.listen).toHaveBeenCalledTimes(5);

    await act(async () => {
      resolveSecondA(staleAUnlisten);
      await startA;
    });
    expect(mocks.cancelMfa).toHaveBeenCalledTimes(1);
    expect(bUnlisten).not.toHaveBeenCalled();

    unmount();
    expect(mocks.cancelMfa).toHaveBeenCalledTimes(2);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('task-b');
    expect(bUnlisten).toHaveBeenCalledTimes(3);
  });

  it('cancels the active polling task and listeners on unmount', async () => {
    const unlisten = vi.fn();
    mocks.listen.mockResolvedValue(unlisten);
    const { result, unmount } = renderHook(() => useMfaOidcConnect());

    await act(async () => {
      await result.current.start();
    });
    unmount();

    expect(mocks.cancelMfa).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('task-1');
    expect(unlisten).toHaveBeenCalledTimes(3);
  });

  it('cancels a late polling task without installing listeners after unmount', async () => {
    let resolvePoll!: (taskId: string) => void;
    mocks.mfaPollOpenId.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolvePoll = resolve;
        }),
    );
    const { result, unmount } = renderHook(() => useMfaOidcConnect());
    let startPromise: Promise<void> | undefined;

    await act(async () => {
      startPromise = result.current.start();
      await Promise.resolve();
    });
    await waitFor(() => expect(mocks.mfaPollOpenId).toHaveBeenCalledTimes(1));

    unmount();
    await act(async () => {
      resolvePoll('late-task');
      await startPromise;
    });

    expect(mocks.cancelMfa).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('late-task');
    expect(mocks.listen).not.toHaveBeenCalled();
  });
});
