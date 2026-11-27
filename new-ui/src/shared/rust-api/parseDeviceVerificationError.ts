import type { DeviceVerificationError } from './types';

export const parseDeviceVerificationError = (
  err: unknown,
): DeviceVerificationError | null => {
  try {
    const parsed = JSON.parse(String(err)) as DeviceVerificationError;
    return parsed && typeof parsed.type === 'string' ? parsed : null;
  } catch {
    return null;
  }
};
