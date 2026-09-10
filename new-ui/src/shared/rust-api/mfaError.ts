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
 *  The backend maps non-cap HTTP 403 responses to `posture_rejected`; ordinary
 *  MFA rejections stay `mfa_rejected`. */
export const isMfaPostureError = (err: unknown, location: LocationInfo): boolean =>
  location.posture_check_required && parseMfaError(err)?.type === 'posture_rejected';

/** The MFA attempt limit was reached and the session must be restarted. */
export const isAttemptLimit = (err: unknown): boolean =>
  parseMfaError(err)?.type === 'attempt_limit';

/** The submitted proof belongs to an older MFA step attempt. */
export const isStaleAttempt = (message: string): boolean =>
  message.includes('stale MFA attempt');

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
 *  Maps to `MfaError::NetworkError` (type: "network_error") and
 *  `MfaError::ProxyError` (type: "proxy_error") from the Rust backend. */
export const isServiceUnavailable = (err: unknown): boolean => {
  const parsed = parseMfaError(err);
  if (!parsed) return false;
  return parsed.type === 'network_error' || parsed.type === 'proxy_error';
};

/** MFA succeeded but bringing up the VPN connection afterwards failed
 *  (see `connect_after_mfa` in the Rust backend). */
export const isConnectFailure = (message: string): boolean =>
  message.includes('VPN connection failed');

/** Why an external-OIDC poll failed, with the message to show for it. */
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

/** Classify an `mfa-openid-error` payload once, so both OIDC hooks agree on the ordering of the
 *  checks and on the wording.
 *
 *  The `kind` is returned alongside the message because the two hooks do not treat every case the
 *  same way: the compact view shows a message for an expired session, while the full view hands
 *  that case to its `onSessionExpired` callback. Callers switch on `kind` only where they diverge
 *  and use `message` everywhere else. */
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
