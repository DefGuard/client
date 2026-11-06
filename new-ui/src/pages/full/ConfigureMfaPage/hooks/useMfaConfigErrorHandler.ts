import { error as logError } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef } from 'react';
import {
  isMfaConfigAlreadyAuthorized,
  isMfaConfigCancelled,
  isMfaConfigFailedPrecondition,
  isMfaConfigForbidden,
  isMfaConfigInvalidCode,
  isMfaConfigMethodNotConfigured,
  isMfaConfigNetworkError,
  isMfaConfigProxyError,
  isMfaConfigSecurityKeyError,
  isMfaConfigSessionExpired,
  isMfaConfigSmtpNotConfigured,
  isMfaConfigTimeout,
  mfaErrorMessage,
} from '../../../../shared/rust-api/mfaError';
import { showEdgeComsError } from '../components/EdgeComsError/useEdgeComsErrorStore';
import { showMfaConfigErrorModal } from './showMfaConfigErrorModal';

type Options = {
  context: string;
  setError: (message: string) => void;
  onSessionExpired: () => void;
  /** Copy for anything untagged, never the raw string, which may be a Rust message. */
  fallback: string;
  /** Whether this step has a code field. Without one, a rejection gets Defguard's own message
   *  instead, since "Invalid code" would point at an input the user cannot see. */
  hasCodeInput?: boolean;
};

/** Shared `MfaConfigError` classification, so every step of the flow reacts the same way. */
export const useMfaConfigErrorHandler = ({
  context,
  setError,
  onSessionExpired,
  fallback,
  hasCodeInput = true,
}: Options) => {
  const dismissModalRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    return () => {
      dismissModalRef.current?.();
    };
  }, []);

  return useCallback(
    (err: unknown, retry?: () => void) => {
      // A cancel is the user's own doing, so it is neither logged nor shown.
      if (isMfaConfigCancelled(err)) {
        return;
      }
      void logError(`${context}: ${err}`);
      if (isMfaConfigInvalidCode(err)) {
        setError(hasCodeInput ? 'Invalid code' : mfaErrorMessage(err));
        return;
      }
      // an OpenID login as another user lands here too, Core ends the session for it
      if (isMfaConfigSessionExpired(err)) {
        setError('Configuration session expired, start again.');
        onSessionExpired();
        return;
      }
      if (isMfaConfigAlreadyAuthorized(err)) {
        setError('This session was already verified, start again.');
        onSessionExpired();
        return;
      }
      if (isMfaConfigMethodNotConfigured(err)) {
        setError('This method is no longer set up for your account.');
        return;
      }
      if (isMfaConfigForbidden(err)) {
        setError(mfaErrorMessage(err));
        return;
      }
      // a consumed or replaced challenge, the next attempt fetches a fresh one
      if (isMfaConfigFailedPrecondition(err)) {
        setError('Verification expired, try again.');
        return;
      }
      if (isMfaConfigTimeout(err)) {
        setError('Sign-in timed out, try again.');
        return;
      }
      // The backend writes these for the user (no key, wrong PIN, no touch), so show as is.
      if (isMfaConfigSecurityKeyError(err)) {
        setError(mfaErrorMessage(err));
        return;
      }
      if (isMfaConfigNetworkError(err)) {
        showEdgeComsError(retry);
        return;
      }
      let message = fallback;
      if (isMfaConfigSmtpNotConfigured(err)) {
        message =
          'Email MFA is unavailable because SMTP is not configured on this Defguard instance. Contact your administrator.';
      } else if (isMfaConfigProxyError(err)) {
        message =
          'Could not complete MFA configuration. Please try again or contact your administrator.';
      }
      dismissModalRef.current = showMfaConfigErrorModal(message);
    },
    [context, setError, onSessionExpired, fallback, hasCodeInput],
  );
};
