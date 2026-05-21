import { useMutation, useQuery } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';

const MFA_ENDPOINT = 'api/v1/client-mfa';
const POLL_INTERVAL_MS = 5_000;
const POLL_TIMEOUT_MS = 5 * 60 * 1_000; // 5 minutes

type MfaStartResponse = { token: string };
type MfaFinishResponse = { preshared_key: string };
type MfaErrorResponse = { error: string };

export const useMfaOidcConnect = () => {
  const { location, setView } = useLocationCardContext();

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [pollError, setPollError] = useState<string | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location.instance_id);

  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const { mutate: connectMutate } = useMutation({
    mutationFn: api.connect,
    onSuccess: () => {
      setView(LocationCardViews.Connected);
    },
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

  // Clean up on unmount
  useEffect(() => {
    return () => {
      stopPolling();
    };
  }, [stopPolling]);

  const startPolling = useCallback(
    (token: string, proxyUrl: string, headers: Record<string, string>) => {
      setIsPolling(true);
      setPollError(null);

      const poll = async () => {
        try {
          const res = await fetch(`${proxyUrl}${MFA_ENDPOINT}/finish`, {
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

          // 428 Precondition Required: user hasn't completed browser auth yet, keep polling
          if (res.status === 428) return;

          // Any other error: stop polling and surface the error
          stopPolling();
          setIsPolling(false);
          const data = (await res.json()) as MfaErrorResponse;
          const msg = data.error;
          if (msg === 'invalid token' || msg === 'login session not found') {
            setPollError('Session expired. Please try again.');
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
    [location, connectMutate, stopPolling],
  );

  const start = useCallback(async () => {
    if (!instance) {
      setStartError('Instance not found');
      return;
    }

    setIsStarting(true);
    setStartError(null);
    setPollError(null);
    stopPolling();

    let headers: Record<string, string>;
    try {
      headers = await api.getEdgeRequestHeaders();
    } catch {
      setStartError('Failed to load request headers');
      setIsStarting(false);
      return;
    }

    let posture_data: unknown;
    try {
      posture_data = location.posture_check_required
        ? await api.getPostureData()
        : undefined;
    } catch {
      setStartError('Failed to load posture data');
      setIsStarting(false);
      return;
    }

    try {
      const res = await fetch(`${instance.proxy_url}${MFA_ENDPOINT}/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...headers },
        body: JSON.stringify({
          method: 2,
          pubkey: instance.pubkey,
          location_id: location.network_id,
          posture_data,
        }),
      });

      if (res.ok) {
        const data = (await res.json()) as MfaStartResponse;
        await api.openLink(`${instance.proxy_url}openid/mfa?token=${data.token}`);
        startPolling(data.token, instance.proxy_url, headers);
      } else {
        const data = (await res.json()) as MfaErrorResponse;
        setStartError(data.error ?? 'Failed to start OIDC authentication');
        error(`OIDC MFA start failed for location ${location.id}: ${data.error}`);
      }
    } catch (e) {
      setStartError('Failed to reach server');
      error(`OIDC MFA start network error for location ${location.id}: ${e}`);
    } finally {
      setIsStarting(false);
    }
  }, [instance, location, startPolling, stopPolling]);

  return { start, isStarting, startError, isPolling, pollError };
};
