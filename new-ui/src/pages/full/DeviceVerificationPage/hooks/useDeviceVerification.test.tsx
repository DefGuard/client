import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import type { PropsWithChildren } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  type DeviceVerificationErrorPayload,
  type DeviceVerificationInfo,
  type DeviceVerificationResolvedPayload,
  TauriEvent,
} from '../../../../shared/rust-api/types';
import { type DeviceVerificationStart, startVerification } from '../startVerification';
import { useDeviceVerification } from './useDeviceVerification';

type Handler = (event: { payload: unknown }) => void;

const mocks = vi.hoisted(() => ({
  listen: vi.fn(),
  startDeviceVerification: vi.fn(),
  getDeviceVerificationCode: vi.fn(),
  handlers: new Map<string, Handler>(),
}));

vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('../../../../shared/rust-api/api', () => ({
  api: {
    startDeviceVerification: mocks.startDeviceVerification,
    getDeviceVerificationCode: mocks.getDeviceVerificationCode,
  },
}));

const INSTANCE_ID = 3;

const info: DeviceVerificationInfo = {
  session_id: 11,
  digits: 6,
  period: 60,
  expires_at: 2_000_000_000,
  code: { code: '755224', expires_in: 42 },
};

const started: DeviceVerificationStart = { ok: true, info, fetchedAt: Date.now() };

const wrapper = ({ children }: PropsWithChildren) => (
  <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
);

const emit = (event: string, payload: unknown) => {
  act(() => {
    mocks.handlers.get(event)?.({ payload });
  });
};

const resolved = (
  overrides: Partial<DeviceVerificationResolvedPayload>,
): DeviceVerificationResolvedPayload => ({
  instance_id: INSTANCE_ID,
  session_id: info.session_id,
  outcome: 'verified',
  resolved_at: 1,
  ...overrides,
});

const render = async (start: DeviceVerificationStart = started) => {
  const onVerified = vi.fn();
  const onBlocked = vi.fn();
  const hook = renderHook(
    () =>
      useDeviceVerification({ instanceId: INSTANCE_ID, start, onVerified, onBlocked }),
    { wrapper },
  );
  // Let the event listeners register.
  await act(async () => {});
  return { hook, onVerified, onBlocked };
};

describe('startVerification', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns the session or the parsed error, never rejects', async () => {
    mocks.startDeviceVerification.mockResolvedValue(info);
    expect(await startVerification(INSTANCE_ID)).toMatchObject({ ok: true, info });

    mocks.startDeviceVerification.mockRejectedValue('{"type":"license_required"}');
    expect(await startVerification(INSTANCE_ID)).toEqual({
      ok: false,
      error: { type: 'license_required' },
    });

    mocks.startDeviceVerification.mockRejectedValue('Instance not found');
    expect(await startVerification(INSTANCE_ID)).toEqual({
      ok: false,
      error: { type: 'other', message: 'Instance not found' },
    });
  });
});

describe('useDeviceVerification', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.handlers.clear();
    mocks.listen.mockImplementation(async (event: string, handler: Handler) => {
      mocks.handlers.set(event, handler);
      return vi.fn();
    });
    mocks.getDeviceVerificationCode.mockResolvedValue(info.code);
  });

  it('shows the code from the loader right away', async () => {
    const { hook } = await render();

    expect(hook.result.current.code).toBe('755224');
    expect(hook.result.current.status).toEqual({ kind: 'pending' });
    expect(hook.result.current.secondsLeft).toBeGreaterThan(40);
    // The first code comes from the loader, refreshing waits for it to expire.
    expect(mocks.getDeviceVerificationCode).not.toHaveBeenCalled();
  });

  it('calls onVerified when the administrator verifies the device', async () => {
    const { onVerified, onBlocked } = await render();

    emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'verified' }));

    expect(onVerified).toHaveBeenCalledTimes(1);
    expect(onBlocked).not.toHaveBeenCalled();
  });

  it('calls onBlocked for a blocked outcome and a blocked wait', async () => {
    const { onBlocked } = await render();

    emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'blocked' }));
    emit(TauriEvent.DeviceVerificationError, {
      instance_id: INSTANCE_ID,
      session_id: info.session_id,
      error: { type: 'device_blocked' },
    } satisfies DeviceVerificationErrorPayload);

    expect(onBlocked).toHaveBeenCalledTimes(2);
  });

  it('moves to the stopped and expired states', async () => {
    const { hook } = await render();

    emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'cancelled' }));
    expect(hook.result.current.status).toEqual({ kind: 'stopped' });
    expect(hook.result.current.code).toBeNull();

    emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'expired' }));
    expect(hook.result.current.status).toEqual({ kind: 'expired' });
  });

  it('ignores events of other instances', async () => {
    const { hook, onVerified } = await render();

    emit(
      TauriEvent.DeviceVerificationResolved,
      resolved({ instance_id: INSTANCE_ID + 1, outcome: 'verified' }),
    );
    emit(TauriEvent.DeviceVerificationError, {
      instance_id: INSTANCE_ID + 1,
      session_id: 1,
      error: { type: 'network', message: 'down' },
    } satisfies DeviceVerificationErrorPayload);

    expect(onVerified).not.toHaveBeenCalled();
    expect(hook.result.current.status).toEqual({ kind: 'pending' });
  });

  it('reports a failed start and wait errors', async () => {
    const failed = await render({ ok: false, error: { type: 'license_required' } });
    expect(failed.hook.result.current.status).toEqual({
      kind: 'error',
      error: { type: 'license_required' },
    });
    expect(failed.hook.result.current.code).toBeNull();
    expect(mocks.getDeviceVerificationCode).not.toHaveBeenCalled();
    failed.hook.unmount();

    const waiting = await render();
    emit(TauriEvent.DeviceVerificationError, {
      instance_id: INSTANCE_ID,
      session_id: info.session_id,
      error: { type: 'proxy', status: 502, message: 'core timeout' },
    } satisfies DeviceVerificationErrorPayload);
    expect(waiting.hook.result.current.status).toEqual({
      kind: 'error',
      error: { type: 'proxy', status: 502, message: 'core timeout' },
    });
  });
});
