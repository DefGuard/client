import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';
import { useConfigureVerifyOidc } from './useConfigureVerifyOidc';

const mocks = vi.hoisted(() => ({
  error: vi.fn(),
  mfaConfigAbortAttempt: vi.fn(),
  mfaConfigAuthorizeOidc: vi.fn(),
  mfaConfigOidcUrl: vi.fn(),
  openLink: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-log', () => ({ error: mocks.error }));
vi.mock('../../../../../shared/rust-api/api', () => ({
  api: {
    mfaConfigAbortAttempt: mocks.mfaConfigAbortAttempt,
    mfaConfigAuthorizeOidc: mocks.mfaConfigAuthorizeOidc,
    mfaConfigOidcUrl: mocks.mfaConfigOidcUrl,
    openLink: mocks.openLink,
  },
}));

const sessionId = 'session-1';
const oidcUrl = 'https://idp.test/auth';
const authorizeResult = { deadline_timestamp: 1_900_000_000, recovery_codes: [] };

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
};

const renderOidc = (autoStart = false) =>
  renderHook(() => useConfigureVerifyOidc({ onSessionExpired: vi.fn(), autoStart }));

describe('useConfigureVerifyOidc', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useConfigureMfaStore.getState().reset();
    useConfigureMfaStore.setState({ sessionId });
    mocks.mfaConfigAbortAttempt.mockResolvedValue(undefined);
    mocks.mfaConfigAuthorizeOidc.mockResolvedValue(authorizeResult);
    mocks.mfaConfigOidcUrl.mockResolvedValue(oidcUrl);
    mocks.openLink.mockResolvedValue(undefined);
  });

  it('opens the browser and polls', async () => {
    const { result } = renderOidc();

    await act(async () => {
      await result.current.start();
    });

    expect(mocks.openLink).toHaveBeenCalledWith(oidcUrl);
    expect(mocks.mfaConfigAuthorizeOidc).toHaveBeenCalledWith(sessionId);
  });

  it('reopens the browser while polling without starting a second poll', async () => {
    const polling = deferred<typeof authorizeResult>();
    mocks.mfaConfigAuthorizeOidc.mockReturnValue(polling.promise);
    const { result } = renderOidc();

    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      await result.current.start();
    });

    expect(mocks.openLink).toHaveBeenCalledTimes(2);
    expect(mocks.mfaConfigAuthorizeOidc).toHaveBeenCalledTimes(1);

    await act(async () => {
      polling.resolve(authorizeResult);
    });
  });

  it('opens the browser on mount when auto-starting', async () => {
    renderOidc(true);

    await waitFor(() => {
      expect(mocks.openLink).toHaveBeenCalledWith(oidcUrl);
    });
    expect(mocks.mfaConfigOidcUrl).toHaveBeenCalledTimes(1);
    expect(mocks.mfaConfigAuthorizeOidc).toHaveBeenCalledWith(sessionId);
  });

  it('does not auto-start when unmounted before the deferred start', async () => {
    const { unmount } = renderOidc(true);
    unmount();
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(mocks.mfaConfigOidcUrl).not.toHaveBeenCalled();
  });

  it('does not open the browser when unmounted while the URL is pending', async () => {
    const fetching = deferred<string>();
    mocks.mfaConfigOidcUrl.mockReturnValue(fetching.promise);
    const { result, unmount } = renderOidc();

    let starting: Promise<void> | undefined;
    act(() => {
      starting = result.current.start();
    });
    unmount();
    fetching.resolve(oidcUrl);
    await starting;

    expect(mocks.openLink).not.toHaveBeenCalled();
    expect(mocks.mfaConfigAuthorizeOidc).not.toHaveBeenCalled();
  });
});
