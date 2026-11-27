import { api } from '../../../../shared/rust-api/api';
import type { DeviceVerificationStart } from '../types';
import { toDeviceVerificationError } from './toDeviceVerificationError';

/** Never rejects, the page shows a failed start as its error state. */
export const startVerification = async (
  instanceId: number,
): Promise<DeviceVerificationStart> => {
  try {
    const info = await api.startDeviceVerification(instanceId);
    return { ok: true, info, fetchedAt: Date.now() };
  } catch (err) {
    return { ok: false, error: toDeviceVerificationError(err) };
  }
};
