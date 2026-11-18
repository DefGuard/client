import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TauriCommand } from '../../../../shared/rust-api/types';
import { activateUser } from './activateUser';

const mocks = vi.hoisted(() => ({
  error: vi.fn(),
  info: vi.fn(),
  enrollmentActivateUser: vi.fn(),
  enrollmentNetworkInfo: vi.fn(),
  invoke: vi.fn(),
  state: {
    sessionId: 'session-1',
    skipPassword: false,
    userPassword: 'password',
    startResponse: { instance: { id: 'server-instance' } } as {
      instance: { id: string };
    } | null,
  },
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/plugin-log', () => ({ error: mocks.error, info: mocks.info }));
vi.mock('../../../../shared/rust-api/api', () => ({
  api: {
    enrollmentActivateUser: mocks.enrollmentActivateUser,
    enrollmentNetworkInfo: mocks.enrollmentNetworkInfo,
  },
}));
vi.mock('./useEnrollmentStore', () => ({
  useEnrollmentStore: { getState: () => mocks.state },
}));

const instances = [
  { id: 1, uuid: 'another-instance', pubkey: 'other-key' },
  { id: 2, uuid: 'server-instance', pubkey: 'enrollment-key' },
];
const networkInfoResponse = { mfa_configured_methods: ['totp'] };

describe('activateUser', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.state.startResponse = { instance: { id: 'server-instance' } };
    mocks.enrollmentActivateUser.mockResolvedValue(undefined);
    mocks.enrollmentNetworkInfo.mockResolvedValue(networkInfoResponse);
    mocks.invoke.mockResolvedValue(instances);
  });

  it('refreshes the matching instance after activation', async () => {
    await expect(activateUser()).resolves.toBeUndefined();

    expect(mocks.info).toHaveBeenNthCalledWith(
      1,
      'Enrollment activation succeeded; starting instance refresh',
    );
    expect(mocks.info).toHaveBeenCalledWith(
      'Enrollment instance lookup matched a saved instance',
    );
    expect(mocks.info).toHaveBeenCalledWith('Calling enrollment network info command');
    expect(mocks.info).toHaveBeenCalledWith('Enrollment instance refresh completed');
    expect(mocks.enrollmentActivateUser).toHaveBeenCalledWith(
      'session-1',
      'password',
      null,
    );
    expect(mocks.enrollmentNetworkInfo).toHaveBeenCalledWith(
      'session-1',
      'enrollment-key',
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(1, TauriCommand.AllInstances);
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, TauriCommand.UpdateInstance, {
      instanceId: 2,
      response: networkInfoResponse,
    });
    expect(mocks.enrollmentActivateUser.mock.invocationCallOrder[0]).toBeLessThan(
      mocks.enrollmentNetworkInfo.mock.invocationCallOrder[0],
    );
    expect(mocks.enrollmentNetworkInfo.mock.invocationCallOrder[0]).toBeLessThan(
      mocks.invoke.mock.invocationCallOrder[1],
    );
  });

  it('resolves when fetching network info fails', async () => {
    mocks.enrollmentNetworkInfo.mockRejectedValueOnce(new Error('offline'));

    await expect(activateUser()).resolves.toBeUndefined();

    expect(mocks.enrollmentActivateUser).toHaveBeenCalledOnce();
    expect(mocks.error).toHaveBeenCalledWith(
      'Failed to refresh instance after enrollment activation: Error: offline',
    );
    expect(mocks.invoke).toHaveBeenCalledOnce();
  });

  it('logs when no saved instance matches the enrollment UUID', async () => {
    mocks.invoke.mockResolvedValueOnce([]);

    await expect(activateUser()).resolves.toBeUndefined();

    expect(mocks.info).toHaveBeenCalledWith(
      'Enrollment instance lookup did not match a saved instance',
    );
    expect(mocks.enrollmentNetworkInfo).not.toHaveBeenCalled();
    expect(mocks.error).toHaveBeenCalledWith(
      'Failed to refresh instance after enrollment activation: Error: Enrollment instance was not found',
    );
  });

  it('logs when the enrollment start response is missing', async () => {
    mocks.state.startResponse = null;

    await expect(activateUser()).resolves.toBeUndefined();

    expect(mocks.error).toHaveBeenCalledWith(
      'Failed to refresh instance after enrollment activation: Error: Enrollment start response is missing',
    );
    expect(mocks.enrollmentNetworkInfo).not.toHaveBeenCalled();
  });
});
