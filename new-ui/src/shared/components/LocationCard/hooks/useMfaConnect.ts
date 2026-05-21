import { useMutation, useQuery } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { EdgeRequestHeaders } from '../../../rust-api/types';
import {
  CLIENT_MFA_ENDPOINT,
  shouldShowPostureError,
  startClientMfaSession,
} from '../api/startClientMfaSession';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';

type MfaFinishResponse = {
  preshared_key: string;
};

type MfaErrorResponse = {
  error: string;
};

export const useMfaConnect = (method: 0 | 1) => {
  const { location, setPostureError, setView } = useLocationCardContext();

  const [token, setToken] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isVerifying, setIsVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState<string | null>(null);
  const [requestHeaders, setRequestHeaders] = useState<EdgeRequestHeaders | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);

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

  // Fire the /start request exactly once when instance data is ready.
  const startCalled = useRef(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional one-shot trigger via startCalled ref
  useEffect(() => {
    if (!instance || startCalled.current) return;
    startCalled.current = true;

    setIsStarting(true);

    (async () => {
      try {
        const { response, headers } = await startClientMfaSession({
          instance,
          location,
          method,
        });
        setRequestHeaders(headers);
        setToken(response.token);
      } catch (err) {
        if (shouldShowPostureError(err, location)) {
          setPostureError(err.message);
          setView(LocationCardViews.PostureCheckFail);
          return;
        }
        setStartError(err instanceof Error ? err.message : 'Failed to start MFA');
      } finally {
        setIsStarting(false);
      }
    })();
  }, [instance]);

  const verifyCode = useCallback(
    async (code: string) => {
      if (!token || !instance || !requestHeaders) return;

      setIsVerifying(true);
      setVerifyError(null);

      const body = JSON.stringify({ token, code });

      try {
        const res = await fetch(`${instance.proxy_url}${CLIENT_MFA_ENDPOINT}/finish`, {
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
    [token, instance, requestHeaders, location, connectMutate, setView],
  );

  return { token, isStarting, startError, verifyCode, isVerifying, verifyError };
};
