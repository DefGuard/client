import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IconKind } from '../../../Icon';
import { DEFAULT_CONNECTION_ERROR } from '../../api/connectError';
import { type LocationCardError, LocationCardViews } from '../../context/types';
import { LocationCardErrorView } from './LocationCardErrorView';

const mocks = vi.hoisted(() => ({
  setView: vi.fn(),
  cardError: null as LocationCardError | null,
  location: { active: false },
}));

vi.mock('../../context/context', () => ({
  useLocationCardContext: () => mocks,
}));

vi.mock('../../api/connectError', () => ({
  DEFAULT_CONNECTION_ERROR: 'Service unavailable',
}));

describe('LocationCardErrorView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.cardError = null;
    mocks.location.active = false;
  });

  it('shows the error icon and message', () => {
    mocks.cardError = {
      message:
        'Your Defguard mobile app is out of date.\nUpgrade to approve this connection.',
      icon: IconKind.DisableMfa,
    };
    const { container } = render(<LocationCardErrorView />);

    expect(container.querySelector('[data-kind="disable-mfa"]')).not.toBeNull();
    expect(container.querySelector('.description')?.textContent).toBe(
      mocks.cardError.message,
    );
  });

  it('falls back to the service error and a secondary Got it button', () => {
    const { container } = render(<LocationCardErrorView />);

    expect(container.querySelector('[data-kind="service-unavailable"]')).not.toBeNull();
    expect(screen.getByText(DEFAULT_CONNECTION_ERROR)).toBeDefined();
    expect(screen.queryByRole('button', { name: 'Retry' })).toBeNull();
    const button = screen.getByRole('button', { name: 'Got it' });
    expect(button.getAttribute('data-variant')).toBe('secondary');
    fireEvent.click(button);
    expect(mocks.setView).toHaveBeenCalledWith(LocationCardViews.Default);
  });

  it('offers Retry instead of Got it when the error can be retried', () => {
    const onRetry = vi.fn();
    mocks.cardError = { message: 'Timed out', icon: IconKind.WarningOutlined, onRetry };
    render(<LocationCardErrorView />);

    expect(screen.queryByRole('button', { name: 'Got it' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('goes back to the connected view for an active location', () => {
    mocks.location.active = true;
    render(<LocationCardErrorView />);

    fireEvent.click(screen.getByRole('button', { name: 'Got it' }));
    expect(mocks.setView).toHaveBeenCalledWith(LocationCardViews.Connected);
  });
});
