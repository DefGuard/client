import { act, renderHook } from '@testing-library/react';
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

const SESSION_ID = 'session-1';
const OIDC_URL = 'https://idp.test/auth';
const authorizeResult = { deadline_timestamp: 1_900_000_000, recovery_codes: [] };

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
};

const renderOidc = () =>
  renderHook(() => useConfigureVerifyOidc({ onSessionExpired: vi.fn() }));

describe('useConfigureVerifyOidc', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useConfigureMfaStore.getState().reset();
    useConfigureMfaStore.setState({ sessionId: SESSION_ID });
    mocks.mfaConfigAbortAttempt.mockResolvedValue(undefined);
    mocks.mfaConfigAuthorizeOidc.mockResolvedValue(authorizeResult);
    mocks.mfaConfigOidcUrl.mockResolvedValue(OIDC_URL);
    mocks.openLink.mockResolvedValue(undefined);
  });

  it('opens the browser and polls', async () => {
    const { result } = renderOidc();

    await act(async () => {
      await result.current.start();
    });

    expect(mocks.openLink).toHaveBeenCalledWith(OIDC_URL);
    expect(mocks.mfaConfigAuthorizeOidc).toHaveBeenCalledWith(SESSION_ID);
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
    fetching.resolve(OIDC_URL);
    await starting;

    expect(mocks.openLink).not.toHaveBeenCalled();
    expect(mocks.mfaConfigAuthorizeOidc).not.toHaveBeenCalled();
  });
});
