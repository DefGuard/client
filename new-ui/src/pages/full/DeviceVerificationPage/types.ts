import type {
  DeviceVerificationError,
  DeviceVerificationInfo,
} from '../../../shared/rust-api/types';

export type DeviceVerificationStart =
  /** fetchedAt anchors the countdown of the first code. */
  | { ok: true; info: DeviceVerificationInfo; fetchedAt: number }
  | { ok: false; error: DeviceVerificationError };
