import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IconKind } from '../../../Icon';
import type { IconKindValue } from '../../../Icon/icon-types';
import { DEFAULT_CONNECTION_ERROR } from '../../api/connectError';
import { LocationCardViews } from '../../context/types';
import { LocationCardConnectionErrorView } from './LocationCardConnectionErrorView';

const mocks = vi.hoisted(() => ({
  setView: vi.fn(),
  connectionError: null as string | null,
  connectionErrorIcon: null as IconKindValue | null,
}));

vi.mock('../../context/context', () => ({
  useLocationCardContext: () => mocks,
}));

vi.mock('../../api/connectError', () => ({
  DEFAULT_CONNECTION_ERROR: 'Service unavailable',
}));

describe('LocationCardConnectionErrorView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.connectionError = null;
    mocks.connectionErrorIcon = null;
  });

  it('shows the mobile error icon and message', () => {
    mocks.connectionError =
      'Your Defguard mobile app is out of date.\nUpgrade to approve this connection.';
    mocks.connectionErrorIcon = IconKind.DisableMfa;
    const { container } = render(<LocationCardConnectionErrorView />);

    expect(container.querySelector('[data-kind="disable-mfa"]')).not.toBeNull();
    expect(container.querySelector('.description')?.textContent).toBe(
      mocks.connectionError,
    );
  });

  it('keeps the service error icon and uses the secondary Got it button', () => {
    mocks.connectionErrorIcon = IconKind.ServiceUnavailable;
    const { container } = render(<LocationCardConnectionErrorView />);

    expect(container.querySelector('[data-kind="service-unavailable"]')).not.toBeNull();
    expect(screen.getByText(DEFAULT_CONNECTION_ERROR)).toBeDefined();
    const button = screen.getByRole('button', { name: 'Got it' });
    expect(button.getAttribute('data-variant')).toBe('secondary');
    fireEvent.click(button);
    expect(mocks.setView).toHaveBeenCalledWith(LocationCardViews.Default);
  });

  it('omits the icon when none is supplied', () => {
    const { container } = render(<LocationCardConnectionErrorView />);

    expect(container.querySelector('.icon')).toBeNull();
  });
});
