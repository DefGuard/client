import { useMutation, useQuery } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import {
  CLIENT_MFA_ENDPOINT,
  MfaStartMethod,
  shouldShowPostureError,
  startClientMfaSession,
} from '../../../../../../shared/components/LocationCard/api/startClientMfaSession';
import { api } from '../../../../../../shared/rust-api/api';
import { getInstancesQueryOptions } from '../../../../../../shared/rust-api/query';
import { useConnectModal } from './useConnectModal';

const POLL_INTERVAL_MS = 5_000;
const POLL_TIMEOUT_MS = 5 * 60 * 1_000;

type MfaFinishResponse = { preshared_key: string };
type MfaErrorResponse = { error: string };

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

  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const { mutate: connectMutate } = useMutation({
    mutationFn: api.connect,
    onError: (err) => {
      error(`Connect command failed after successful OIDC MFA\n${err}`);
      setPollError('Failed to establish VPN connection');
    },
  });

  const stopPolling = useCallback(() => {
    if (intervalRef.current !== null) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  useEffect(() => {
    return () => {
      stopPolling();
    };
  }, [stopPolling]);

  const startPolling = useCallback(
    (token: string, proxyUrl: string, headers: Record<string, string>) => {
      if (!location) return;

      setIsPolling(true);
      setPollError(null);

      const poll = async () => {
        try {
          const res = await fetch(`${proxyUrl}${CLIENT_MFA_ENDPOINT}/finish`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...headers },
            body: JSON.stringify({ token }),
          });

          if (res.ok) {
            stopPolling();
            setIsPolling(false);
            const data = (await res.json()) as MfaFinishResponse;
            connectMutate({
              locationId: location.id,
              connectionType: location.connection_type,
              presharedKey: data.preshared_key,
            });
            return;
          }

          if (res.status === 428) return;

          stopPolling();
          setIsPolling(false);
          const data = (await res.json()) as MfaErrorResponse;
          const msg = data.error;
          if (msg === 'invalid token' || msg === 'login session not found') {
            onSessionExpired?.();
          } else {
            setPollError('Authentication failed. Please try again.');
          }
          error(`OIDC MFA poll failed for location ${location.id}: ${msg}`);
        } catch (e) {
          stopPolling();
          setIsPolling(false);
          setPollError('Failed to reach server. Please try again.');
          error(`OIDC MFA poll network error for location ${location.id}: ${e}`);
        }
      };

      intervalRef.current = setInterval(poll, POLL_INTERVAL_MS);

      timeoutRef.current = setTimeout(() => {
        stopPolling();
        setIsPolling(false);
        setPollError('Authentication timed out. Please try again.');
        error(`OIDC MFA timed out for location ${location.id}`);
      }, POLL_TIMEOUT_MS);
    },
    [location, connectMutate, stopPolling, onSessionExpired],
  );

  const start = useCallback(async () => {
    if (!instance || !location) {
      setStartError('Instance not found');
      return;
    }

    setIsStarting(true);
    setStartError(null);
    setPollError(null);
    stopPolling();

    try {
      const { response, headers } = await startClientMfaSession({
        instance,
        location,
        method: MfaStartMethod.Oidc,
      });
      await api.openLink(`${instance.proxy_url}openid/mfa?token=${response.token}`);
      startPolling(response.token, instance.proxy_url, headers);
    } catch (e) {
      if (shouldShowPostureError(e, location)) {
        onPostureError?.(e.message);
        return;
      }
      setStartError(
        e instanceof Error ? e.message : 'Failed to start OIDC authentication',
      );
      error(`OIDC MFA start network error for location ${location.id}: ${e}`);
    } finally {
      setIsStarting(false);
    }
  }, [instance, location, startPolling, stopPolling, onPostureError]);

  return { start, isStarting, startError, isPolling, pollError };
};
