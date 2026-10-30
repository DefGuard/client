import {
  isAttemptLimit,
  isConnectFailure,
  isMobileApprovalRejected,
  isMobileClientOutdated,
  isSecurityKeyPinBlocked,
  isServiceUnavailable,
  isSessionExpired,
  isStaleAttempt,
  isTimeout,
  MfaFailureText,
  mfaErrorMessage,
  parseMfaError,
} from '../../../rust-api/mfaError';
import type { LocationInfo } from '../../../rust-api/types';
import { IconKind } from '../../Icon';
import type { LocationCardError } from '../context/types';
import { type ConnectError, DEFAULT_CONNECTION_ERROR } from './connectError';

export const MfaRetry = {
  // the hooks keep the session token for these errors, so the step can resume
  ResumeStep: 'resume-step',
  RestartMfa: 'restart-mfa',
} as const;

export type MfaRetryValue = (typeof MfaRetry)[keyof typeof MfaRetry];

export type MfaFailure = Omit<LocationCardError, 'onRetry'> & {
  retry: MfaRetryValue | null;
};

export const mfaErrorToCard = (err: unknown): MfaFailure => {
  if (isServiceUnavailable(err)) {
    return {
      message: DEFAULT_CONNECTION_ERROR,
      icon: IconKind.ServiceUnavailable,
      retry: MfaRetry.ResumeStep,
    };
  }
  if (isTimeout(err)) {
    return {
      message: MfaFailureText.Timeout,
      icon: IconKind.WarningOutlined,
      retry: MfaRetry.ResumeStep,
    };
  }
  const message = mfaErrorMessage(err);
  if (isMobileClientOutdated(err)) {
    return { message, icon: IconKind.DisableMfa, retry: null };
  }
  if (
    isAttemptLimit(err) ||
    isMobileApprovalRejected(message) ||
    isSecurityKeyPinBlocked(message)
  ) {
    return { message, icon: IconKind.WarningOutlined, retry: null };
  }
  const restart = (text: string): MfaFailure => ({
    message: text,
    icon: IconKind.WarningOutlined,
    retry: MfaRetry.RestartMfa,
  });
  if (isStaleAttempt(message)) return restart(MfaFailureText.StaleAttempt);
  if (isSessionExpired(message)) return restart(MfaFailureText.SessionExpired);
  if (isConnectFailure(message)) return restart(MfaFailureText.ConnectFailure);
  // only structured errors carry a message worded for the user
  return restart(parseMfaError(err)?.message ?? MfaFailureText.Unknown);
};

export const connectErrorToCard = (
  connectError: ConnectError | null,
  location: LocationInfo,
): LocationCardError => {
  if (connectError?.kind === 'serviceUnavailable') {
    return { message: DEFAULT_CONNECTION_ERROR, icon: IconKind.ServiceUnavailable };
  }
  return {
    message: `Failed to connect to ${location.name}. Please try again.`,
    icon: IconKind.WarningOutlined,
  };
};
