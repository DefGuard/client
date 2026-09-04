import { encode } from '@stablelib/base64';
import { useQuery } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import {
  isCancelled,
  isConnectFailure,
  isMfaPostureError,
  isServiceUnavailable,
  mfaErrorMessage,
} from '../../../rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { LocationInfo, MfaErrorPayload } from '../../../rust-api/types';
import { MfaMethod, TauriEvent } from '../../../rust-api/types';
import { isPresent } from '../../../utils/isPresent';

type TokenData = {
  token: string;
  challenge: string;
};

type Options = {
  onConnected?: () => void;
  onPostureError?: (message?: string) => void;
  onServiceUnavailable?: () => void;
};

export const useMfaMobileConnect = (location: LocationInfo, options?: Options) => {
  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location.instance_id);
  const instanceId = instance?.id;
  const instanceUuid = instance?.uuid;
  const locationId = location.id;

  // Read through refs so callers passing inline callbacks or a re-fetched
  // `location` object don't restart the mobile-approve task on every render.
  const optionsRef = useRef(options);
  optionsRef.current = options;
  const locationRef = useRef(location);
  locationRef.current = location;

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [tokenData, setTokenData] = useState<TokenData | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);

  const taskIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const cleanupListeners = useCallback(() => {
    if (unlistenRef.current !== null) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      cleanupListeners();
      const taskId = taskIdRef.current;
      if (taskId) {
        void api.cancelMfa(taskId).catch(() => {});
      }
    };
  }, [cleanupListeners]);

  // Connect WebSocket via Rust when tokenData is available
  useEffect(() => {
    if (!tokenData || !isPresent(instanceId)) return;

    let cancelled = false;
    cleanupListeners();
    setIsConnecting(true);
    setConnectionError(null);

    (async () => {
      try {
        const taskId = await api.mfaConnectMobileApprove(
          instanceId,
          locationId,
          tokenData.token,
        );
        if (cancelled) {
          void api.cancelMfa(taskId).catch(() => {});
          return;
        }
        taskIdRef.current = taskId;

        // The backend brings up the connection itself; completion means connected.
        const completeUnlisten = await listen(TauriEvent.MfaMobileComplete, () => {
          cleanupListeners();
          taskIdRef.current = null;
          setIsConnecting(false);
          optionsRef.current?.onConnected?.();
        });

        const errorUnlisten = await listen<MfaErrorPayload>(
          TauriEvent.MfaMobileError,
          (event) => {
            // Emitted app-wide without a task id, so our own cancels land here.
            if (isCancelled(event.payload.error)) return;
            cleanupListeners();
            taskIdRef.current = null;
            setIsConnecting(false);
            error(`Mobile MFA failed for location ${locationId}: ${event.payload.error}`);
            const message = mfaErrorMessage(event.payload.error);
            setConnectionError(
              isConnectFailure(message)
                ? 'Failed to establish VPN connection'
                : 'Connection error. Please try again.',
            );
          },
        );

        unlistenRef.current = () => {
          completeUnlisten();
          errorUnlisten();
        };
      } catch (e) {
        if (!cancelled) {
          setIsConnecting(false);
          setConnectionError('Failed to start mobile approval. Please try again.');
          error(`Mobile MFA connect failed for location ${locationId}: ${e}`);
        }
      }
    })();

    return () => {
      cancelled = true;
      cleanupListeners();
      const taskId = taskIdRef.current;
      if (taskId) {
        taskIdRef.current = null;
        void api.cancelMfa(taskId).catch(() => {});
      }
      setIsConnecting(false);
    };
  }, [tokenData, instanceId, locationId, cleanupListeners]);

  const qrValue = useMemo(() => {
    if (!tokenData || !isPresent(instanceUuid)) return null;
    const json = JSON.stringify({
      token: tokenData.token,
      challenge: tokenData.challenge,
      instance_id: instanceUuid,
    });
    return encode(new TextEncoder().encode(json));
  }, [tokenData, instanceUuid]);

  const start = useCallback(async () => {
    if (!isPresent(instanceId)) {
      setStartError('Instance not found');
      return;
    }

    setIsStarting(true);
    setStartError(null);
    setConnectionError(null);
    // Clear previous task via effect
    setTokenData(null);

    try {
      const info = await api.mfaStart(instanceId, locationId, MfaMethod.MobileApprove);
      if (!info.challenge) {
        setStartError('Unsupported response from proxy');
        return;
      }

      setTokenData({ token: info.token, challenge: info.challenge });
    } catch (e) {
      void error(`Mobile MFA start failed for location ${locationId}: ${e}`);
      if (isMfaPostureError(e, locationRef.current)) {
        optionsRef.current?.onPostureError?.(mfaErrorMessage(e));
        return;
      }
      if (isServiceUnavailable(e)) {
        optionsRef.current?.onServiceUnavailable?.();
        return;
      }
      setStartError(mfaErrorMessage(e));
    } finally {
      setIsStarting(false);
    }
  }, [instanceId, locationId]);

  const reset = useCallback(() => {
    cleanupListeners();
    const taskId = taskIdRef.current;
    if (taskId) {
      void api.cancelMfa(taskId).catch(() => {});
      taskIdRef.current = null;
    }
    setTokenData(null);
    setIsStarting(false);
    setStartError(null);
    setIsConnecting(false);
    setConnectionError(null);
  }, [cleanupListeners]);

  return { start, isStarting, startError, qrValue, isConnecting, connectionError, reset };
};
