import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ButtonVariant } from '../../../../shared/components/Button/types';
import { useConfirmModal } from '../../../../shared/hooks/confirmModal/useConfirmModal';
import { showEdgeComsError } from '../components/EdgeComsError/useEdgeComsErrorStore';
import { useMfaConfigErrorHandler } from './useMfaConfigErrorHandler';

vi.mock('@tauri-apps/plugin-log', () => ({ error: vi.fn() }));
vi.mock('../components/EdgeComsError/useEdgeComsErrorStore', () => ({
  showEdgeComsError: vi.fn(),
}));

const renderHandler = (hasCodeInput = true) => {
  const setError = vi.fn();
  const onSessionExpired = vi.fn();
  const hook = renderHook(() =>
    useMfaConfigErrorHandler({
      context: 'Email MFA configuration failed',
      setError,
      onSessionExpired,
      fallback: 'Configuration failed',
      hasCodeInput,
    }),
  );
  return { ...hook, setError, onSessionExpired };
};

const error = (type: string, message?: string) => JSON.stringify({ type, message });

describe('useMfaConfigErrorHandler', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useConfirmModal.getState().reset();
  });

  it('shows missing SMTP in an informational modal instead of the code input', () => {
    const { result, setError, onSessionExpired } = renderHandler();
    const retry = vi.fn();
    act(() => result.current(error('smtp_not_configured'), retry));

    expect(useConfirmModal.getState()).toMatchObject({
      visible: true,
      title: 'MFA configuration error',
      content:
        'Email MFA is unavailable because SMTP is not configured on this Defguard instance. Contact your administrator.',
      cancelProps: { text: 'Got it', variant: ButtonVariant.Primary },
      submitProps: null,
    });
    expect(setError).not.toHaveBeenCalled();
    expect(onSessionExpired).not.toHaveBeenCalled();
    expect(retry).not.toHaveBeenCalled();
  });

  it('shows a safe modal for a server error without exposing backend details', () => {
    const { result, setError } = renderHandler();
    act(() =>
      result.current(
        JSON.stringify({ type: 'proxy_error', status: 500, message: 'private details' }),
      ),
    );

    expect(useConfirmModal.getState()).toMatchObject({
      visible: true,
      content:
        'Could not complete MFA configuration. Please try again or contact your administrator.',
    });
    expect(setError).not.toHaveBeenCalled();
  });

  it.each([new Error('private details'), error('future_error', 'private details')])(
    'uses the caller fallback in a modal for an unknown error',
    (err) => {
      const { result, setError } = renderHandler();
      act(() => result.current(err));

      expect(useConfirmModal.getState()).toMatchObject({
        visible: true,
        content: 'Configuration failed',
      });
      expect(setError).not.toHaveBeenCalled();
    },
  );

  it('keeps invalid codes inline', () => {
    const { result, setError } = renderHandler();
    act(() => result.current(error('invalid_code', 'invalid code')));

    expect(setError).toHaveBeenCalledWith('Invalid code');
    expect(useConfirmModal.getState().visible).toBe(false);
  });

  it('preserves rejections without a code input', () => {
    const { result, setError } = renderHandler(false);
    act(() => result.current(error('invalid_code', 'Verification rejected')));

    expect(setError).toHaveBeenCalledWith('Verification rejected');
    expect(useConfirmModal.getState().visible).toBe(false);
  });

  it('preserves session expiration handling', () => {
    const { result, onSessionExpired } = renderHandler();
    act(() => result.current(error('session_expired')));

    expect(onSessionExpired).toHaveBeenCalledOnce();
    expect(useConfirmModal.getState().visible).toBe(false);
  });

  it('preserves the network error screen and retry callback', () => {
    const { result, setError } = renderHandler();
    const retry = vi.fn();
    act(() => result.current(error('network_error'), retry));

    expect(showEdgeComsError).toHaveBeenCalledWith(retry);
    expect(setError).not.toHaveBeenCalled();
    expect(useConfirmModal.getState().visible).toBe(false);
  });

  it('ignores cancellation', () => {
    const { result, setError, onSessionExpired } = renderHandler();
    act(() => result.current(error('cancelled')));

    expect(setError).not.toHaveBeenCalled();
    expect(onSessionExpired).not.toHaveBeenCalled();
    expect(useConfirmModal.getState().visible).toBe(false);
  });

  it('dismisses its modal when the step unmounts', () => {
    const { result, unmount } = renderHandler();
    act(() => result.current(error('smtp_not_configured')));
    unmount();

    expect(useConfirmModal.getState().visible).toBe(false);
  });

  it('does not dismiss a modal opened by another caller', () => {
    const { result, unmount } = renderHandler();
    act(() => result.current(error('smtp_not_configured')));
    act(() => useConfirmModal.getState().open({ title: 'Another modal' }));
    unmount();

    expect(useConfirmModal.getState()).toMatchObject({
      visible: true,
      title: 'Another modal',
    });
  });
});
