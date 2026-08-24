import { useQuery } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import {
  isConnectFailure,
  isMfaPostureError,
  isServiceUnavailable,
  isSessionExpired,
  isTimeout,
  mfaErrorMessage,
} from '../../../rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { MfaErrorPayload } from '../../../rust-api/types';
import { MfaMethod, TauriEvent } from '../../../rust-api/types';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';

export const useMfaOidcConnect = () => {
  const { location, setPostureError, setView, stepPlan, mfaToken, setMfaToken } =
    useLocationCardContext();

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [pollError, setPollError] = useState<string | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location.instance_id);

  const taskIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const cleanup = useCallback(() => {
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
      const session = await api.startMfaStep(
        instance.id,
        location.id,
        MfaMethod.Oidc,
        stepPlan,
        mfaToken,
      );
      setMfaToken(session.token);

      await api.openLink(`${instance.proxy_url}openid/mfa?token=${session.token}`);

      setIsStarting(false);
      setIsPolling(true);

      const taskId = await api.mfaPollOpenId(instance.id, location.id, session.token);
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
          error(`OIDC MFA failed for location ${location.id}: ${event.payload.error}`);
          const message = mfaErrorMessage(event.payload.error);
          if (isTimeout(event.payload.error)) {
            setPollError('Authentication timed out. Please try again.');
          } else if (isConnectFailure(message)) {
            setPollError('Failed to establish VPN connection');
          } else if (isSessionExpired(message)) {
            setPollError('Session expired. Please try again.');
          } else {
            setPollError('Authentication failed. Please try again.');
          }
        },
      );

      unlistenRef.current = () => {
        completeUnlisten();
        errorUnlisten();
      };
    } catch (e) {
      void error(`OIDC MFA start failed for location ${location.id}: ${e}`);
      if (isMfaPostureError(e, location)) {
        setPostureError(mfaErrorMessage(e));
        setView(LocationCardViews.PostureCheckFail);
        return;
      }
      if (isServiceUnavailable(e)) {
        setView(LocationCardViews.ConnectionError);
        return;
      }
      setStartError(mfaErrorMessage(e));
    } finally {
      setIsStarting(false);
    }
  }, [
    instance,
    location,
    stepPlan,
    mfaToken,
    setMfaToken,
    setPostureError,
    setView,
    cleanup,
  ]);

  return { start, isStarting, startError, isPolling, pollError };
};
