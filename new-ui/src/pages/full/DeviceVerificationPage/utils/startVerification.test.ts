import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DeviceVerificationInfo } from '../../../../shared/rust-api/types';
import { startVerification } from './startVerification';

const mocks = vi.hoisted(() => ({
  startDeviceVerification: vi.fn(),
}));

vi.mock('../../../../shared/rust-api/api', () => ({
  api: { startDeviceVerification: mocks.startDeviceVerification },
}));

const instanceId = 3;

const info: DeviceVerificationInfo = {
  session_id: 11,
  digits: 6,
  period: 60,
  expires_at: 2_000_000_000,
  code: { code: '755224', expires_in: 42 },
};

describe('startVerification', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns the session or the parsed error, never rejects', async () => {
    mocks.startDeviceVerification.mockResolvedValue(info);
    expect(await startVerification(instanceId)).toMatchObject({ ok: true, info });

    mocks.startDeviceVerification.mockRejectedValue('{"type":"license_required"}');
    expect(await startVerification(instanceId)).toEqual({
      ok: false,
      error: { type: 'license_required' },
    });

    mocks.startDeviceVerification.mockRejectedValue('Instance not found');
    expect(await startVerification(instanceId)).toEqual({
      ok: false,
      error: { type: 'other', message: 'Instance not found' },
    });
  });
});
