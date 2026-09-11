import type { LocationInfo } from './types';

/** JSON error returned by the Rust backend. */
export type ParsedMfaError = {
  type: string;
  message?: string;
  status?: number;
};

/** Parses a JSON MFA error, or returns null for plain text. */
export const parseMfaError = (err: unknown): ParsedMfaError | null => {
  try {
    const parsed = JSON.parse(String(err)) as ParsedMfaError;
    return parsed && typeof parsed.type === 'string' ? parsed : null;
  } catch {
    return null;
  }
};

/** Returns the error message, or the original error text. */
export const mfaErrorMessage = (err: unknown): string =>
  parseMfaError(err)?.message ?? String(err);

/** Returns true when MFA was rejected by the device posture check. */
export const isMfaPostureError = (err: unknown, location: LocationInfo): boolean =>
  location.posture_check_required && parseMfaError(err)?.type === 'posture_rejected';

/** The MFA attempt limit was reached and the session must be restarted. */
export const isAttemptLimit = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'attempt_limit';

/** The submitted proof belongs to an older MFA step attempt. */
export const isStaleAttempt = (message: string): boolean =>
  message.includes('stale MFA attempt');

/** The Edge session/token is no longer valid. */
export const isSessionExpired = (message: string): boolean =>
  message.includes('invalid token') || message.includes('login session not found');

/** The MFA operation timed out. */
export const isTimeout = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'timeout';

/** A submitted one-time code was rejected. */
export const isInvalidCode = (message: string): boolean =>
  message.includes('Unauthorized');

/** Returns true when the Edge service is unavailable. */
export const isServiceUnavailable = (err: unknown): boolean => {
  const parsed = parseMfaError(err);
  if (!parsed) return false;
  return parsed.type === 'network_error' || parsed.type === 'proxy_error';
};

/** Returns true when MFA succeeded but the VPN connection failed. */
export const isConnectFailure = (message: string): boolean =>
  message.includes('VPN connection failed');

/** Describes an OIDC polling error and its user-facing message. */
export type OidcPollFailure = {
  kind:
    | 'attemptLimit'
    | 'staleAttempt'
    | 'timeout'
    | 'connectFailure'
    | 'sessionExpired'
    | 'unknown';
  message: string;
};

/** Maps an OIDC error event to a type and user-facing message. */
export const classifyOidcPollFailure = (rawError: string): OidcPollFailure => {
  const message = mfaErrorMessage(rawError);
  if (isAttemptLimit(rawError)) {
    return { kind: 'attemptLimit', message };
  }
  if (isStaleAttempt(message)) {
    return {
      kind: 'staleAttempt',
      message: 'Authentication request could not be started. Please try again.',
    };
  }
  if (isTimeout(rawError)) {
    return { kind: 'timeout', message: 'Authentication timed out. Please try again.' };
  }
  if (isConnectFailure(message)) {
    return { kind: 'connectFailure', message: 'Failed to establish VPN connection' };
  }
  if (isSessionExpired(message)) {
    return { kind: 'sessionExpired', message: 'Session expired. Please try again.' };
  }
  return { kind: 'unknown', message: 'Authentication failed. Please try again.' };
};
