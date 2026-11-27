import { parseDeviceVerificationError } from '../../../../shared/rust-api/parseDeviceVerificationError';
import type { DeviceVerificationError } from '../../../../shared/rust-api/types';

export const toDeviceVerificationError = (err: unknown): DeviceVerificationError => {
  return parseDeviceVerificationError(err) ?? { type: 'other', message: String(err) };
};
