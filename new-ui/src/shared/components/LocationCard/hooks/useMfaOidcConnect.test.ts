import { act, renderHook } from '@testing-library/react';
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
      step_attempt_id: 'attempt-1',
      token: 'mfa-token',
    });
    mocks.mfaPollOpenId.mockResolvedValue('task-1');
    mocks.openLink.mockResolvedValue(undefined);
    mocks.listen.mockResolvedValue(vi.fn());
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
});
