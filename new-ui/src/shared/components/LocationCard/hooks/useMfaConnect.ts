import { useMutation, useQuery } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import {
  getInstancesQueryOptions,
  getPlatformHeaderQueryOptions,
} from '../../../rust-api/query';
import type { EdgeRequestHeaders } from '../../../rust-api/types';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';

const MFA_ENDPOINT = 'api/v1/client-mfa';

type MfaStartResponse = {
  token: string;
  challenge?: string;
};

type MfaFinishResponse = {
  preshared_key: string;
};

type MfaErrorResponse = {
  error: string;
};

export const useMfaConnect = (method: 0 | 1) => {
  const { location, setView } = useLocationCardContext();

  const [token, setToken] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isVerifying, setIsVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState<string | null>(null);
  const [requestHeaders, setRequestHeaders] = useState<EdgeRequestHeaders | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const { data: platformHeader } = useQuery(getPlatformHeaderQueryOptions);

  const instance = instances?.find((i) => i.id === location.instance_id);

  const { mutate: connectMutate } = useMutation({
    mutationFn: api.connect,
    meta: { invalidate: ['locations'] },
    onSuccess: () => {
      setView(LocationCardViews.Connected);
    },
    onError: (err) => {
      error(`Connect command failed after successful code verification\n${err}`);
    },
  });

  // Fire the /start request exactly once when instance + platformHeader are ready.
  const startCalled = useRef(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional one-shot trigger via startCalled ref
  useEffect(() => {
    if (!instance || !platformHeader || startCalled.current) return;
    startCalled.current = true;

    setIsStarting(true);

    (async () => {
      let headers: EdgeRequestHeaders;
      try {
        headers = await api.getEdgeRequestHeaders();
        setRequestHeaders(headers);
      } catch {
        setStartError('Failed to load request headers');
        setIsStarting(false);
        return;
      }

      try {
        const res = await fetch(`${instance.proxy_url}${MFA_ENDPOINT}/start`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...headers,
          },
          body: JSON.stringify({
            method,
            pubkey: instance.pubkey,
            location_id: location.network_id,
          }),
        });

        if (res.ok) {
          const data = (await res.json()) as MfaStartResponse;
          setToken(data.token);
        } else {
          const data = (await res.json()) as MfaErrorResponse;
          setStartError(data.error ?? 'Failed to start MFA');
        }
      } catch {
        setStartError('Failed to reach server');
      } finally {
        setIsStarting(false);
      }
    })();
  }, [instance, platformHeader]);

  const verifyCode = useCallback(
    async (code: string) => {
      if (!token || !instance || !platformHeader || !requestHeaders) return;

      setIsVerifying(true);
      setVerifyError(null);

      const body = JSON.stringify({ token, code });

      try {
        const res = await fetch(`${instance.proxy_url}${MFA_ENDPOINT}/finish`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...requestHeaders,
          },
          body,
        });

        if (res.ok) {
          const data = (await res.json()) as MfaFinishResponse;
          connectMutate({
            locationId: location.id,
            connectionType: location.connection_type,
            presharedKey: data.preshared_key,
          });
        } else {
          const data = (await res.json()) as MfaErrorResponse;
          const { error: errorMessage } = data;
          if (errorMessage === 'Unauthorized') {
            setVerifyError('Invalid code');
          } else if (
            errorMessage === 'invalid token' ||
            errorMessage === 'login session not found'
          ) {
            setView(LocationCardViews.Default);
          } else {
            setVerifyError('Verification failed');
          }
        }
      } catch {
        setVerifyError('Failed to reach server');
      } finally {
        setIsVerifying(false);
      }
    },
    [token, instance, platformHeader, requestHeaders, location, connectMutate, setView],
  );

  return { token, isStarting, startError, verifyCode, isVerifying, verifyError };
};
