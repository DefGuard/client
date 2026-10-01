import { error as logError } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../../../shared/rust-api/api';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  applyAuthorization,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import { useMfaConfigErrorHandler } from '../../hooks/useMfaConfigErrorHandler';

type Options = {
  onSessionExpired: () => void;
};

/** reopening the page starts a new attempt on Core, and the running poll picks it up */
export const useConfigureVerifyOidc = ({ onSessionExpired }: Options) => {
  const [isOpening, setIsOpening] = useState(false);
  const [isPolling, setIsPolling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pollingRef = useRef(false);
  const mountedRef = useRef(true);

  const handleApiError = useMfaConfigErrorHandler({
    context: 'OpenID MFA configuration verification failed',
    setError,
    onSessionExpired,
    fallback: 'Verification failed',
    hasCodeInput: false,
  });

  const poll = useCallback(
    async (sessionId: string) => {
      pollingRef.current = true;
      setIsPolling(true);
      try {
        const result = await api.mfaConfigAuthorizeOidc(sessionId);
        applyAuthorization(sessionId, result);
      } catch (err) {
        if (mountedRef.current) handleApiError(err);
      } finally {
        pollingRef.current = false;
        if (mountedRef.current) setIsPolling(false);
      }
    },
    [handleApiError],
  );

  const start = useCallback(async () => {
    const { sessionId } = useConfigureMfaStore.getState();
    if (!isPresent(sessionId)) return;
    setIsOpening(true);
    setError(null);
    try {
      const url = await api.mfaConfigOidcUrl(sessionId);
      if (!mountedRef.current) return;
      await api.openLink(url);
    } catch (err) {
      if (mountedRef.current) handleApiError(err);
      return;
    } finally {
      if (mountedRef.current) setIsOpening(false);
    }
    if (mountedRef.current && !pollingRef.current) void poll(sessionId);
  }, [handleApiError, poll]);

  const abort = useCallback(async () => {
    const { sessionId } = useConfigureMfaStore.getState();
    if (!pollingRef.current || !isPresent(sessionId)) return;
    try {
      await api.mfaConfigAbortAttempt(sessionId);
    } catch (err) {
      void logError(`Failed to abort OpenID verification: ${err}`);
    }
  }, []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: aborts on unmount only
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      void abort();
    };
  }, []);

  return { start, abort, isOpening, isPolling, error };
};
