import { api } from '../../../shared/rust-api/api';
import { parseDeviceVerificationError } from '../../../shared/rust-api/parseDeviceVerificationError';
import type {
  DeviceVerificationError,
  DeviceVerificationInfo,
} from '../../../shared/rust-api/types';

export const DEVICE_BLOCKED_MESSAGE =
  'This device has been blocked by your administrator.';

export type DeviceVerificationStart =
  /** fetchedAt anchors the countdown of the first code. */
  | { ok: true; info: DeviceVerificationInfo; fetchedAt: number }
  | { ok: false; error: DeviceVerificationError };

export const toDeviceVerificationError = (err: unknown): DeviceVerificationError =>
  parseDeviceVerificationError(err) ?? { type: 'other', message: String(err) };

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
