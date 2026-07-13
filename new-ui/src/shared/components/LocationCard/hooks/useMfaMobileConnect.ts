import { encode } from '@stablelib/base64';
import { useQuery } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { LocationInfo, MfaErrorPayload } from '../../../rust-api/types';
import { MfaMethod, TauriEvent } from '../../../rust-api/types';

type TokenData = {
  token: string;
  challenge: string;
};

const isMfaPostureError = (err: unknown, location: LocationInfo): boolean => {
  if (!location.posture_check_required) return false;
  try {
    const parsed = JSON.parse(String(err)) as { type?: string };
    return parsed.type === 'mfaRejected';
  } catch {
    return false;
  }
};

type Options = {
  onConnected?: () => void;
  onPostureError?: (message?: string) => void;
};

export const useMfaMobileConnect = (location: LocationInfo, options?: Options) => {
  const { onConnected, onPostureError } = options ?? {};

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location.instance_id);

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
    if (!tokenData || !instance) return;

    let cancelled = false;
    cleanupListeners();
    setIsConnecting(true);
    setConnectionError(null);

    (async () => {
      try {
        const taskId = await api.mfaConnectMobileApprove(
          instance.id,
          location.id,
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
          setIsConnecting(false);
          onConnected?.();
        });

        const errorUnlisten = await listen<MfaErrorPayload>(
          TauriEvent.MfaMobileError,
          (event) => {
            cleanupListeners();
            setIsConnecting(false);
            setConnectionError('Connection error. Please try again.');
            error(
              `Mobile MFA failed for location ${location.id}: ${event.payload.error}`,
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
          error(`Mobile MFA connect failed for location ${location.id}: ${e}`);
        }
      }
    })();

    return () => {
      cancelled = true;
      cleanupListeners();
      setIsConnecting(false);
    };
  }, [tokenData, instance, location, onConnected, cleanupListeners]);

  const qrValue = useMemo(() => {
    if (!tokenData || !instance) return null;
    const json = JSON.stringify({
      token: tokenData.token,
      challenge: tokenData.challenge,
      instance_id: instance.uuid,
    });
    return encode(new TextEncoder().encode(json));
  }, [tokenData, instance]);

  const start = useCallback(async () => {
    if (!instance) {
      setStartError('Instance not found');
      return;
    }

    setIsStarting(true);
    setStartError(null);
    setConnectionError(null);
    // Clear previous task via effect
    setTokenData(null);

    try {
      const info = await api.mfaStart(instance.id, location.id, MfaMethod.MobileApprove);
      if (!info.challenge) {
        setStartError('Unsupported response from proxy');
        return;
      }

      setTokenData({ token: info.token, challenge: info.challenge });
    } catch (e) {
      void error(`Mobile MFA start failed for location ${location.id}: ${e}`);
      if (isMfaPostureError(e, location)) {
        onPostureError?.(String(e));
        return;
      }
      setStartError(String(e));
    } finally {
      setIsStarting(false);
    }
  }, [instance, location, onPostureError]);

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
