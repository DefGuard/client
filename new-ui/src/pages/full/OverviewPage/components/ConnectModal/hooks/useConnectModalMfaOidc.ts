import { useQuery } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { api } from '../../../../../../shared/rust-api/api';
import {
  isConnectFailure,
  isMfaPostureError,
  isSessionExpired,
  isTimeout,
  mfaErrorMessage,
} from '../../../../../../shared/rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../../../../shared/rust-api/query';
import type { MfaErrorPayload } from '../../../../../../shared/rust-api/types';
import { MfaMethod, TauriEvent } from '../../../../../../shared/rust-api/types';
import { useConnectModal } from './useConnectModal';

type Options = {
  onPostureError?: (msg: string) => void;
  onSessionExpired?: () => void;
};

export const useConnectModalMfaOidc = ({
  onPostureError,
  onSessionExpired,
}: Options = {}) => {
  const location = useConnectModal(useShallow((s) => s.location));

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [pollError, setPollError] = useState<string | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location?.instance_id);

  const taskIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const cleanup = useCallback(() => {
    if (unlistenRef.current !== null) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

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
    if (!instance || !location) {
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
            onSessionExpired?.();
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
        onPostureError?.(mfaErrorMessage(e));
        return;
      }
      setStartError(mfaErrorMessage(e));
    } finally {
      setIsStarting(false);
    }
  }, [instance, location, cleanup, onPostureError, onSessionExpired]);

  return { start, isStarting, startError, isPolling, pollError };
};
