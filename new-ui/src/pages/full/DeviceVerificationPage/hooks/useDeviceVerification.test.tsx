import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import type { PropsWithChildren } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getInstancesQueryOptions } from '../../../../shared/rust-api/query';
import {
  DeviceStatus,
  type DeviceStatusValue,
  type DeviceVerificationErrorPayload,
  type DeviceVerificationInfo,
  type DeviceVerificationResolvedPayload,
  type InstanceInfo,
  TauriEvent,
} from '../../../../shared/rust-api/types';
import type { DeviceVerificationSession } from '../types';
import { useDeviceVerification } from './useDeviceVerification';

type Handler = (event: { payload: unknown }) => void;

const mocks = vi.hoisted(() => ({
  listen: vi.fn(),
  getDeviceVerificationCode: vi.fn(),
  handlers: new Map<string, Handler>(),
}));

vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('../../../../shared/rust-api/api', () => ({
  api: {
    getDeviceVerificationCode: mocks.getDeviceVerificationCode,
  },
}));

const instanceId = 3;

const info: DeviceVerificationInfo = {
  session_id: 11,
  digits: 6,
  period: 60,
  expires_at: 2_000_000_000,
  code: { code: '755224', expires_in: 42 },
};

const started: DeviceVerificationSession = { info, fetchedAt: Date.now() };

const otherInstanceId = instanceId + 1;

const instance = (id: number) =>
  ({ id, device_status: DeviceStatus.Unverified }) as InstanceInfo;

const emit = (event: string, payload: unknown) => {
  act(() => {
    mocks.handlers.get(event)?.({ payload });
  });
};

const resolved = (
  overrides: Partial<DeviceVerificationResolvedPayload>,
): DeviceVerificationResolvedPayload => ({
  instance_id: instanceId,
  session_id: info.session_id,
  outcome: 'verified',
  resolved_at: 1,
  ...overrides,
});

const render = async (start: DeviceVerificationSession = started) => {
  const onVerified = vi.fn();
  const onBlocked = vi.fn();
  const queryClient = new QueryClient();
  queryClient.setQueryData(getInstancesQueryOptions.queryKey, [
    instance(instanceId),
    instance(otherInstanceId),
  ]);
  const wrapper = ({ children }: PropsWithChildren) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  const hook = renderHook(
    () => useDeviceVerification({ instanceId, start, onVerified, onBlocked }),
    { wrapper },
  );
  // Let the event listeners register.
  await act(async () => {});
  const deviceStatuses = () =>
    queryClient
      .getQueryData(getInstancesQueryOptions.queryKey)
      ?.map(({ id, device_status }) => [id, device_status]);
  return { hook, onVerified, onBlocked, deviceStatuses };
};

const expectStatuses = (deviceStatuses: () => unknown, status: DeviceStatusValue) => {
  expect(deviceStatuses()).toEqual([
    [instanceId, status],
    [otherInstanceId, DeviceStatus.Unverified],
  ]);
};

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
  });

  it('checks the session once the listeners are registered', async () => {
    const { hook } = await render();

    expect(mocks.getDeviceVerificationCode).toHaveBeenCalledTimes(1);
    expect(mocks.getDeviceVerificationCode).toHaveBeenCalledWith(instanceId);
    expect(hook.result.current.code).toBe('755224');
    expect(hook.result.current.status).toEqual({ kind: 'pending' });
  });

  it('calls onVerified when the administrator verifies the device', async () => {
    const { onVerified, onBlocked, deviceStatuses } = await render();

    emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'verified' }));

    expect(onVerified).toHaveBeenCalledTimes(1);
    expect(onBlocked).not.toHaveBeenCalled();
    expectStatuses(deviceStatuses, DeviceStatus.Verified);
  });

  it('calls onBlocked for a blocked outcome', async () => {
    const { onBlocked, deviceStatuses } = await render();

    emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'blocked' }));

    expect(onBlocked).toHaveBeenCalledTimes(1);
    expectStatuses(deviceStatuses, DeviceStatus.Blocked);
  });

  it('calls onBlocked for a blocked wait', async () => {
    const { onBlocked, deviceStatuses } = await render();

    emit(TauriEvent.DeviceVerificationError, {
      instance_id: instanceId,
      session_id: info.session_id,
      error: { type: 'device_blocked' },
    } satisfies DeviceVerificationErrorPayload);

    expect(onBlocked).toHaveBeenCalledTimes(1);
    expectStatuses(deviceStatuses, DeviceStatus.Blocked);
  });

  it('moves to the cancelled and expired states', async () => {
    const { hook, deviceStatuses } = await render();

    emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'cancelled' }));
    expect(hook.result.current.status).toEqual({ kind: 'cancelled' });
    expect(hook.result.current.code).toBeNull();

    emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'expired' }));
    expect(hook.result.current.status).toEqual({ kind: 'expired' });
    expectStatuses(deviceStatuses, DeviceStatus.Unverified);
  });

  it('ignores events of other instances', async () => {
    const { hook, onVerified, deviceStatuses } = await render();

    emit(
      TauriEvent.DeviceVerificationResolved,
      resolved({ instance_id: otherInstanceId, outcome: 'verified' }),
    );
    emit(TauriEvent.DeviceVerificationError, {
      instance_id: otherInstanceId,
      session_id: 1,
      error: { type: 'network', message: 'down' },
    } satisfies DeviceVerificationErrorPayload);

    expect(onVerified).not.toHaveBeenCalled();
    expect(hook.result.current.status).toEqual({ kind: 'pending' });
    expectStatuses(deviceStatuses, DeviceStatus.Unverified);
  });

  describe('when the outcome event came before the listeners', () => {
    const endedWith = (outcome: DeviceVerificationResolvedPayload['outcome']) => {
      mocks.getDeviceVerificationCode.mockRejectedValue(
        JSON.stringify({ type: 'resolved', outcome, resolved_at: null }),
      );
    };

    // The loader's code is valid for another 42 seconds, so nothing waits for its rollover.
    it('verifies once, right after mounting', async () => {
      endedWith('verified');
      const { onVerified, deviceStatuses } = await render();

      await vi.waitFor(() => {
        expect(onVerified).toHaveBeenCalled();
      });
      emit(TauriEvent.DeviceVerificationResolved, resolved({ outcome: 'verified' }));

      expect(onVerified).toHaveBeenCalledTimes(1);
      expectStatuses(deviceStatuses, DeviceStatus.Verified);
    });

    it('shows the expired state right after mounting', async () => {
      endedWith('expired');
      const { hook } = await render();

      await vi.waitFor(() => {
        expect(hook.result.current.status).toEqual({ kind: 'expired' });
      });
      expect(hook.result.current.code).toBeNull();
    });
  });

  it('reports wait errors', async () => {
    const waiting = await render();
    emit(TauriEvent.DeviceVerificationError, {
      instance_id: instanceId,
      session_id: info.session_id,
      error: { type: 'proxy', status: 502, message: 'core timeout' },
    } satisfies DeviceVerificationErrorPayload);
    expect(waiting.hook.result.current.status).toEqual({
      kind: 'error',
      error: { type: 'proxy', status: 502, message: 'core timeout' },
    });
    expect(waiting.hook.result.current.code).toBeNull();
  });
});
