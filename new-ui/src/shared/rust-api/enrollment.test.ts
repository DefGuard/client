import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { enrollmentAddInstance, enrollmentCreateDevice } from './enrollment';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const nativeInvoke = vi.mocked(invoke);
const deviceResponse = {
  device: { id: 701 },
  configs: [],
  instance: { id: 'core-uuid' },
};
const savedConfig = { instance: { id: 42 }, locations: [] };

const deferred = <T>() => {
  let resolve: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve: (value: T) => resolve(value) };
};

describe('enrollmentCreateDevice', () => {
  beforeEach(() => {
    nativeInvoke.mockReset();
    nativeInvoke.mockImplementation(async (command) => {
      switch (command) {
        case 'enrollment_create_device':
          return deviceResponse;
        case 'save_device_config':
          return savedConfig;
        case 'enrollment_attest_device':
          return 'attested';
        default:
          throw new Error(`Unexpected command: ${command}`);
      }
    });
  });

  it('attests only after persistence and waits before allowing enrollment to continue', async () => {
    const save = deferred<unknown>();
    const attestation = deferred<unknown>();
    nativeInvoke.mockImplementation(async (command) => {
      if (command === 'enrollment_create_device') return deviceResponse;
      if (command === 'save_device_config') return save.promise;
      if (command === 'enrollment_attest_device') return attestation.promise;
      throw new Error(`Unexpected command: ${command}`);
    });
    let completed = false;
    const result = enrollmentCreateDevice('local-session', 'laptop').then((response) => {
      completed = true;
      return response;
    });
    await vi.waitFor(() => expect(nativeInvoke).toHaveBeenCalledTimes(2));
    expect(completed).toBe(false);
    expect(nativeInvoke.mock.calls[0]).toEqual([
      'enrollment_create_device',
      { sessionId: 'local-session', name: 'laptop', pubkey: expect.any(String) },
    ]);
    expect(nativeInvoke.mock.calls[1]).toEqual([
      'save_device_config',
      { privateKey: expect.any(String), response: deviceResponse },
    ]);
    save.resolve(savedConfig);
    await vi.waitFor(() => expect(nativeInvoke).toHaveBeenCalledTimes(3));
    expect(nativeInvoke.mock.calls[2]).toEqual([
      'enrollment_attest_device',
      { sessionId: 'local-session', instanceId: 42 },
    ]);
    expect(completed).toBe(false);
    attestation.resolve('attested');
    expect(await result).toEqual({});
  });

  it.each(['skipped', 'failed'])(
    'keeps registration successful when attestation is %s',
    async (outcome) => {
      nativeInvoke
        .mockResolvedValueOnce(deviceResponse)
        .mockResolvedValueOnce(savedConfig)
        .mockResolvedValueOnce(outcome);
      expect(await enrollmentCreateDevice('local-session', 'laptop')).toEqual({});
      expect(nativeInvoke).toHaveBeenCalledTimes(3);
    },
  );

  it('keeps registration successful if the attestation command rejects', async () => {
    nativeInvoke
      .mockResolvedValueOnce(deviceResponse)
      .mockResolvedValueOnce(savedConfig)
      .mockRejectedValueOnce(new Error('attestation unavailable'));
    expect(await enrollmentCreateDevice('local-session', 'laptop')).toEqual({});
    expect(nativeInvoke).toHaveBeenCalledTimes(3);
  });

  it('does not save or attest if device creation fails', async () => {
    nativeInvoke.mockRejectedValueOnce(new Error('creation failed'));
    expect(await enrollmentCreateDevice('local-session', 'laptop')).toEqual({
      error: 'creation failed',
    });
    expect(nativeInvoke).toHaveBeenCalledTimes(1);
  });

  it('does not attest if saving the configuration rejects', async () => {
    nativeInvoke
      .mockResolvedValueOnce(deviceResponse)
      .mockRejectedValueOnce(new Error('configuration save failed'));
    expect(await enrollmentCreateDevice('local-session', 'laptop')).toEqual({
      error: 'configuration save failed',
    });
    expect(nativeInvoke).toHaveBeenCalledTimes(2);
  });

  it('does not attest an imported existing device', async () => {
    nativeInvoke.mockImplementation(async (command) => {
      switch (command) {
        case 'enrollment_start':
          return { session_id: 'local-session', instance: { id: 'core-uuid' } };
        case 'all_instances':
          return [{ id: 42, uuid: 'core-uuid', pubkey: 'existing-wg-key' }];
        case 'enrollment_network_info':
          return deviceResponse;
        case 'update_instance':
        case 'enrollment_finish':
          return undefined;
        default:
          throw new Error(`Unexpected command: ${command}`);
      }
    });
    expect(
      await enrollmentAddInstance({
        url: 'https://proxy.example',
        token: 'enrollment-token',
        name: 'laptop',
      }),
    ).toEqual({});
    expect(nativeInvoke.mock.calls.map(([command]) => command)).toEqual([
      'enrollment_start',
      'all_instances',
      'enrollment_network_info',
      'update_instance',
      'enrollment_finish',
    ]);
  });
});
