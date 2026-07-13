import { useQuery } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import {
  isConnectFailure,
  isMfaPostureError,
  isSessionExpired,
  mfaErrorMessage,
} from '../../../rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { MfaErrorPayload } from '../../../rust-api/types';
import { MfaMethod, TauriEvent } from '../../../rust-api/types';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';

const POLL_TIMEOUT_MS = 5 * 60 * 1_000; // 5 minutes

export const useMfaOidcConnect = () => {
  const { location, setPostureError, setView } = useLocationCardContext();

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [pollError, setPollError] = useState<string | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location.instance_id);

  const taskIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const cleanup = useCallback(() => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    if (unlistenRef.current !== null) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      cleanup();
      const taskId = taskIdRef.current;
      if (taskId) {
        void api.cancelMfa(taskId).catch(() => {});
      }
    };
  }, [cleanup]);

  const start = useCallback(async () => {
    if (!instance) {
      setStartError('Instance not found');
      return;
    }

    setIsStarting(true);
    setStartError(null);
    setPollError(null);
    cleanup();

    try {
      const info = await api.mfaStart(instance.id, location.id, MfaMethod.Oidc);
      await api.openLink(`${instance.proxy_url}openid/mfa?token=${info.token}`);

      setIsStarting(false);
      setIsPolling(true);

      const taskId = await api.mfaPollOpenId(instance.id, location.id, info.token);
      taskIdRef.current = taskId;

      // The backend brings up the connection itself; completion means connected.
      const completeUnlisten = await listen(TauriEvent.MfaOpenIdComplete, () => {
        cleanup();
        setIsPolling(false);
        setView(LocationCardViews.Connected);
      });

      const errorUnlisten = await listen<MfaErrorPayload>(
        TauriEvent.MfaOpenIdError,
        (event) => {
          cleanup();
          setIsPolling(false);
          const message = mfaErrorMessage(event.payload.error);
          if (isConnectFailure(message)) {
            setPollError('Failed to establish VPN connection');
          } else if (isSessionExpired(message)) {
            setPollError('Session expired. Please try again.');
          } else {
            setPollError('Authentication failed. Please try again.');
          }
          error(`OIDC MFA failed for location ${location.id}: ${message}`);
        },
      );

      unlistenRef.current = () => {
        completeUnlisten();
        errorUnlisten();
      };

      timeoutRef.current = setTimeout(() => {
        const taskId = taskIdRef.current;
        if (taskId) {
          void api.cancelMfa(taskId).catch(() => {});
        }
        cleanup();
        setIsPolling(false);
        setPollError('Authentication timed out. Please try again.');
        error(`OIDC MFA timed out for location ${location.id}`);
      }, POLL_TIMEOUT_MS);
    } catch (e) {
      void error(`OIDC MFA start failed for location ${location.id}: ${e}`);
      if (isMfaPostureError(e, location)) {
        setPostureError(String(e));
        setView(LocationCardViews.PostureCheckFail);
        return;
      }
      setStartError(String(e));
    } finally {
      setIsStarting(false);
    }
  }, [instance, location, setPostureError, setView, cleanup]);

  return { start, isStarting, startError, isPolling, pollError };
};
