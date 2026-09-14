import { error as logError } from '@tauri-apps/plugin-log';
import { useCallback } from 'react';
import {
  isMfaConfigCancelled,
  isMfaConfigInvalidCode,
  isMfaConfigProxyError,
  isMfaConfigSecurityKeyError,
  isMfaConfigSessionExpired,
  mfaErrorMessage,
} from '../../../../shared/rust-api/mfaError';

type Options = {
  context: string;
  setError: (message: string) => void;
  onSessionExpired: () => void;
  /** Copy for anything untagged, never the raw string, which may be a Rust message. */
  fallback: string;
};

/** Shared `MfaConfigError` classification, so every step of the flow reacts the same way. */
export const useMfaConfigErrorHandler = ({
  context,
  setError,
  onSessionExpired,
  fallback,
}: Options) =>
  useCallback(
    (err: unknown) => {
      // A cancel is the user's own doing, so it is neither logged nor shown.
      if (isMfaConfigCancelled(err)) {
        return;
      }
      void logError(`${context}: ${err}`);
      if (isMfaConfigInvalidCode(err)) {
        setError('Invalid code');
        return;
      }
      if (isMfaConfigSessionExpired(err)) {
        setError('Configuration session expired, start again.');
        onSessionExpired();
        return;
      }
      // The backend writes these for the user (no key, wrong PIN, no touch), so show as is.
      if (isMfaConfigSecurityKeyError(err)) {
        setError(mfaErrorMessage(err));
        return;
      }
      if (isMfaConfigProxyError(err)) {
        setError('Service temporarily unavailable, try again.');
        return;
      }
      setError(fallback);
    },
    [context, setError, onSessionExpired, fallback],
  );
