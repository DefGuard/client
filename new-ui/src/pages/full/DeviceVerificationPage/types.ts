import type {
  DeviceVerificationError,
  DeviceVerificationInfo,
} from '../../../shared/rust-api/types';

export type DeviceVerificationSession = {
  info: DeviceVerificationInfo;
  // anchors the countdown of the first code
  fetchedAt: number;
};

export type DeviceVerificationStart =
  | ({ ok: true } & DeviceVerificationSession)
  | { ok: false; error: DeviceVerificationError };
