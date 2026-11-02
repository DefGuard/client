import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ConfirmModal } from '../../../../shared/components/ConfirmModal/ConfirmModal';
import { useConfirmModal } from '../../../../shared/hooks/confirmModal/useConfirmModal';
import { MfaMethod } from '../../../../shared/rust-api/types';
import { showMfaConfigErrorModal } from './showMfaConfigErrorModal';
import { useConfigureMfaStore } from './useConfigureMfaStore';

vi.hoisted(() => {
  document.body.innerHTML = '<div id="root"></div><div id="modals-root"></div>';
});
vi.mock('@tauri-apps/plugin-log', () => ({ error: vi.fn() }));
vi.mock('../../../../shared/rust-api/api', () => ({ api: {} }));

const message = 'Email MFA is unavailable because SMTP is not configured.';

describe('MFA configuration error modal', () => {
  beforeEach(() => {
    useConfirmModal.getState().reset();
    useConfigureMfaStore.getState().reset();
    useConfigureMfaStore.setState({
      sessionId: 'session-1',
      selectedMethods: [MfaMethod.Email],
      authorized: true,
    });
  });

  it.each(['Got it', 'close button'])(
    'dismisses via %s without resetting the flow',
    (action) => {
      render(
        <QueryClientProvider client={new QueryClient()}>
          <ConfirmModal />
        </QueryClientProvider>,
      );
      act(() => {
        showMfaConfigErrorModal(message);
      });

      expect(screen.getByText('MFA configuration error')).toBeTruthy();
      expect(screen.getByText(message)).toBeTruthy();
      expect(screen.queryByText('Cancel')).toBeNull();
      const button =
        action === 'Got it'
          ? screen.getByRole('button', { name: 'Got it' })
          : screen.getByRole('button', { name: '' });
      fireEvent.click(button);

      expect(useConfirmModal.getState().visible).toBe(false);
      expect(useConfigureMfaStore.getState()).toMatchObject({
        sessionId: 'session-1',
        selectedMethods: [MfaMethod.Email],
        authorized: true,
      });
    },
  );
});
