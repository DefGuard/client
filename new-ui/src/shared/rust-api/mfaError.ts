import type { LocationInfo } from './types';

/** Shape of the tagged `MfaError` the Rust backend serializes to JSON. */
export type ParsedMfaError = {
  type: string;
  message?: string;
  status?: number;
};

/** Parse a structured `MfaError` (JSON) thrown by a command or carried on an
 *  event payload. Returns null for plain-string errors. */
export const parseMfaError = (err: unknown): ParsedMfaError | null => {
  try {
    const parsed = JSON.parse(String(err)) as ParsedMfaError;
    return parsed && typeof parsed.type === 'string' ? parsed : null;
  } catch {
    return null;
  }
};

/** Best-effort human-readable message: the structured `message` when present,
 *  otherwise the raw error string. */
export const mfaErrorMessage = (err: unknown): string =>
  parseMfaError(err)?.message ?? String(err);

/** True when the error is a posture rejection for a posture-gated location.
 *  The backend maps only HTTP 403 (a failed device posture check) to
 *  `posture_rejected`; ordinary MFA rejections stay `mfa_rejected`. */
export const isMfaPostureError = (err: unknown, location: LocationInfo): boolean =>
  location.posture_check_required && parseMfaError(err)?.type === 'posture_rejected';

/** The proxy session/token is no longer valid. */
export const isSessionExpired = (message: string): boolean =>
  message.includes('invalid token') || message.includes('login session not found');

/** The MFA operation timed out (the backend poll deadline was reached). */
export const isTimeout = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'timeout';

/** A submitted one-time code was rejected. */
export const isInvalidCode = (message: string): boolean =>
  message.includes('Unauthorized');

/** The proxy/edge service is unavailable (network error or 5xx response).
 *  Maps to `MfaError::NetworkError` (type: "networkError") and
 *  `MfaError::ProxyError` (type: "proxyError") from the Rust backend. */
export const isServiceUnavailable = (err: unknown): boolean => {
  const parsed = parseMfaError(err);
  if (!parsed) return false;
  return parsed.type === 'networkError' || parsed.type === 'proxyError';
};

/** MFA succeeded but bringing up the VPN connection afterwards failed
 *  (see `connect_after_mfa` in the Rust backend). */
export const isConnectFailure = (message: string): boolean =>
  message.includes('VPN connection failed');
