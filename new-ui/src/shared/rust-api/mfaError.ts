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

/** Wording for errors the backend sends as a bare type, without a message of their own. */
const MESSAGE_BY_TYPE: Record<string, string> = {
  timeout: 'The operation timed out. Please try again.',
  cancelled: 'Authentication was cancelled.',
};

/** Returns the error message, or the original error text when it is not a JSON error. */
export const mfaErrorMessage = (err: unknown): string => {
  const parsed = parseMfaError(err);
  if (!parsed) return String(err);
  return (
    parsed.message ??
    MESSAGE_BY_TYPE[parsed.type] ??
    'Authentication failed. Please try again.'
  );
};

/** The user backed out, which is not worth showing as an error. */
export const isCancelled = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'cancelled';

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

/** The proxy predates the MFA configuration API (HTTP 404 on /mfa-config/start). */
export const isMfaConfigUnsupported = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'unsupported';

export const isMfaConfigSessionExpired = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'session_expired';

export const isMfaConfigInvalidCode = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'invalid_code';

export const isMfaConfigMissingToken = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'no_token';

/** The backend's message names what went wrong (no key, wrong PIN, no touch), so show it as is. */
export const isMfaConfigSecurityKeyError = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'security_key';

/** The command rejects on the user's own cancel, which is not worth showing as an error. */
export const isMfaConfigCancelled = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'cancelled';

/** e.g. the last security key was removed while the session was open */
export const isMfaConfigMethodNotConfigured = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'method_not_configured';

/** e.g. an inactive user or too many attempts, Core words these for the user */
export const isMfaConfigForbidden = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'forbidden';

/** the response that authorized it was lost, so only a new session gets past it */
export const isMfaConfigAlreadyAuthorized = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'already_authorized';

/** e.g. a consumed or replaced FIDO2 challenge, a fresh attempt fixes it */
export const isMfaConfigFailedPrecondition = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'failed_precondition';

/** only the OpenID wait times out */
export const isMfaConfigTimeout = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'timeout';

/** The request never reached the proxy, unlike `proxy_error` where it answered. */
export const isMfaConfigNetworkError = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'network_error';

/** a status with no specific handling, such as 429 or 5xx */
export const isMfaConfigProxyError = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'proxy_error';

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
