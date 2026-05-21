import { encode } from '@stablelib/base64';
import { useMutation } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';

const MFA_ENDPOINT = 'api/v1/client-mfa';

type MfaStartResponse = {
  token: string;
  challenge: string;
};

type MfaErrorResponse = {
  error: string;
};

type TokenData = {
  token: string;
  challenge: string;
};

export const useMfaMobileConnect = () => {
  const { location, instance, setView } = useLocationCardContext();

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [tokenData, setTokenData] = useState<TokenData | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const expectedCloseRef = useRef(false);

  const { mutate: connectMutate } = useMutation({
    mutationFn: api.connect,
    onSuccess: () => {
      setView(LocationCardViews.Connected);
    },
    onError: (err) => {
      error(`Connect command failed after successful mobile MFA\n${err}`);
      setConnectionError('Failed to establish VPN connection');
    },
  });

  // Open WebSocket when tokenData is available
  useEffect(() => {
    if (!tokenData) return;

    const wsUrl = `${instance.proxy_url
      .replace(/^http:/, 'ws:')
      .replace(
        /^https:/,
        'wss:',
      )}${MFA_ENDPOINT}/remote?token=${encodeURIComponent(tokenData.token)}`;

    expectedCloseRef.current = false;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setIsConnecting(true);
      setConnectionError(null);
    };

    ws.onmessage = (event: MessageEvent) => {
      try {
        const parsed = JSON.parse(event.data as string) as unknown;
        if (
          typeof parsed === 'object' &&
          parsed !== null &&
          'preshared_key' in parsed &&
          typeof (parsed as Record<string, unknown>).preshared_key === 'string'
        ) {
          const presharedKey = (parsed as { preshared_key: string }).preshared_key;
          expectedCloseRef.current = true;
          connectMutate({
            locationId: location.id,
            connectionType: location.connection_type,
            presharedKey,
          });
        } else {
          error(
            `Unexpected mobile MFA WS message for location ${location.id}: ${event.data}`,
          );
        }
      } catch (e) {
        error(`Failed to parse mobile MFA WS message for location ${location.id}: ${e}`);
      }
    };

    ws.onerror = () => {
      if (!expectedCloseRef.current) {
        setIsConnecting(false);
        setConnectionError('Connection error. Please try again.');
        error(`Mobile MFA WebSocket error for location ${location.id}`);
      }
    };

    ws.onclose = () => {
      if (!expectedCloseRef.current) {
        setIsConnecting(false);
        setConnectionError('Connection closed unexpectedly. Please try again.');
        error(`Mobile MFA WebSocket closed unexpectedly for location ${location.id}`);
      }
    };

    return () => {
      expectedCloseRef.current = true;
      ws.close();
      wsRef.current = null;
      setIsConnecting(false);
    };
  }, [tokenData, instance, connectMutate, location]);

  // Clean up WebSocket on unmount
  useEffect(() => {
    return () => {
      if (wsRef.current) {
        expectedCloseRef.current = true;
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, []);

  const qrValue = useMemo(() => {
    if (!tokenData) return null;
    const json = JSON.stringify({
      token: tokenData.token,
      challenge: tokenData.challenge,
      instance_id: instance.uuid,
    });
    return encode(new TextEncoder().encode(json));
  }, [tokenData, instance.uuid]);

  const start = useCallback(async () => {
    setIsStarting(true);
    setStartError(null);
    setConnectionError(null);
    // Clear previous token → triggers WS cleanup via effect
    setTokenData(null);

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
          method: 4,
          pubkey: instance.pubkey,
          location_id: location.network_id,
          posture_data,
        }),
      });

      if (res.ok) {
        const data = (await res.json()) as MfaStartResponse;
        setTokenData({ token: data.token, challenge: data.challenge });
      } else {
        const data = (await res.json()) as MfaErrorResponse;
        setStartError(data.error ?? 'Failed to start mobile authentication');
        error(`Mobile MFA start failed for location ${location.id}: ${data.error}`);
      }
    } catch (e) {
      setStartError('Failed to reach server');
      error(`Mobile MFA start network error for location ${location.id}: ${e}`);
    } finally {
      setIsStarting(false);
    }
  }, [instance, location]);

  const reset = useCallback(() => {
    if (wsRef.current) {
      expectedCloseRef.current = true;
      wsRef.current.close();
      wsRef.current = null;
    }
    setTokenData(null);
    setIsStarting(false);
    setStartError(null);
    setIsConnecting(false);
    setConnectionError(null);
  }, []);

  return { start, isStarting, startError, qrValue, isConnecting, connectionError, reset };
};
