import { encode } from '@stablelib/base64';
import { useMutation } from '@tanstack/react-query';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import {
  CLIENT_MFA_ENDPOINT,
  MfaStartMethod,
  startClientMfaSession,
} from '../api/startClientMfaSession';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';
import { handleMfaStartError } from './handleMfaStartError';

type TokenData = {
  token: string;
  challenge: string;
};

export const useMfaMobileConnect = () => {
  const { location, instance, setPostureError, setView } = useLocationCardContext();

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
      )}${CLIENT_MFA_ENDPOINT}/remote?token=${encodeURIComponent(tokenData.token)}`;

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

    try {
      const { response } = await startClientMfaSession({
        instance,
        location,
        method: MfaStartMethod.MobileApprove,
      });
      if (!response.challenge) {
        setStartError('Unsupported response from proxy');
        return;
      }

      setTokenData({ token: response.token, challenge: response.challenge });
    } catch (e) {
      if (handleMfaStartError({ err: e, location, setPostureError, setView })) {
        return;
      }
      setStartError(
        e instanceof Error ? e.message : 'Failed to start mobile authentication',
      );
      error(`Mobile MFA start network error for location ${location.id}: ${e}`);
    } finally {
      setIsStarting(false);
    }
  }, [instance, location, setPostureError, setView]);

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
