import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { error as logError } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../../../shared/rust-api/api';
import { fido2ShowsTouchPrompt } from '../../../../../shared/rust-api/fido2';
import { isMfaConfigInvalidCode } from '../../../../../shared/rust-api/mfaError';
import { TauriEvent } from '../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  applyAuthorization,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import { useMfaConfigErrorHandler } from '../../hooks/useMfaConfigErrorHandler';

type Options = {
  onSessionExpired: () => void;
  autoStart: boolean;
};

/** each verify fetches a fresh single-use challenge, so a retry is just another call */
export const useConfigureVerifyFido2 = ({ onSessionExpired, autoStart }: Options) => {
  const [isVerifying, setIsVerifying] = useState(false);
  const [isAwaitingTouch, setIsAwaitingTouch] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const running = useRef(false);
  const mounted = useRef(true);

  const handleApiError = useMfaConfigErrorHandler({
    context: 'Security key MFA configuration verification failed',
    setError,
    onSessionExpired,
    fallback: 'Verification failed',
    hasCodeInput: false,
  });

  const verify = useCallback(
    async (pin: string | null) => {
      const { sessionId } = useConfigureMfaStore.getState();
      if (running.current || !isPresent(sessionId)) return;
      running.current = true;
      setIsVerifying(true);
      setError(null);
      let unlisten: UnlistenFn | undefined;
      try {
        // listen before invoking, the touch event fires while the call is still running
        unlisten = await listen(TauriEvent.MfaConfigFido2Touch, () => {
          // a platform that runs the ceremony shows its own prompt, ours would sit behind it
          if (mounted.current) setIsAwaitingTouch(fido2ShowsTouchPrompt());
        });
        // an abort sent while listen was pending found no ceremony to stop
        if (!mounted.current) return;
        const result = await api.mfaConfigAuthorizeFido2(sessionId, pin);
        applyAuthorization(sessionId, result);
      } catch (err) {
        if (!mounted.current) return;
        // Core says "invalid code", which means nothing next to a security key
        if (isMfaConfigInvalidCode(err)) {
          void logError(`Security key MFA configuration verification rejected: ${err}`);
          setError('Security key verification failed, try again.');
          return;
        }
        handleApiError(err);
      } finally {
        unlisten?.();
        running.current = false;
        if (mounted.current) {
          setIsAwaitingTouch(false);
          setIsVerifying(false);
        }
      }
    },
    [handleApiError],
  );

  const abort = useCallback(async () => {
    const { sessionId } = useConfigureMfaStore.getState();
    if (!running.current || !isPresent(sessionId)) return;
    try {
      await api.mfaConfigAbortAttempt(sessionId);
    } catch (err) {
      void logError(`Failed to abort security key verification: ${err}`);
    }
  }, []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: aborts on unmount only
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      void abort();
    };
  }, []);

  // deferred a tick so a StrictMode replay or an instant unmount clears it before it runs
  // biome-ignore lint/correctness/useExhaustiveDependencies: auto-start only on mount
  useEffect(() => {
    if (!autoStart) return;
    const timer = window.setTimeout(() => {
      void verify(null);
    }, 0);
    return () => window.clearTimeout(timer);
  }, [autoStart]);

  return { verify, abort, isVerifying, isAwaitingTouch, error, setError };
};
