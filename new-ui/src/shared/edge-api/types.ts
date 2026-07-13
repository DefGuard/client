import type { EnrollmentStartResult } from '../rust-api/types';

/** `network`: the request could not be sent, most likely a bad URL.
 *  `unauthorized`: the server responded 401, the token is invalid.
 *  `server`: any other failure response. */
export type EnrollmentErrorKind = 'network' | 'unauthorized' | 'server';

export type AddInstanceRequest = { url: string; token: string; name: string };
export type AddInstanceResult = {
  startResponse?: EnrollmentStartResult;
  session_id?: string;
  error?: string;
  errorKind?: EnrollmentErrorKind;
};

export type UpdateInstanceRequest = { instanceId: number; url: string; token: string };
export type UpdateInstanceResult = {
  error?: string;
  errorKind?: EnrollmentErrorKind;
};
