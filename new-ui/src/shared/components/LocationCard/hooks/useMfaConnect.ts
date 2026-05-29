import { useMutation, useQuery } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { EdgeRequestHeaders } from '../../../rust-api/types';
import {
  CLIENT_MFA_ENDPOINT,
  type MfaStartMethod,
  startClientMfaSession,
} from '../api/startClientMfaSession';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';
import { handleMfaStartError } from './handleMfaStartError';

type MfaFinishResponse = {
  preshared_key: string;
};

type MfaErrorResponse = {
  error: string;
};

type CodeMfaStartMethod = Extract<MfaStartMethod, 0 | 1>;

type UseMfaConnectOptions = {
  debounceMs?: number;
};

const waitForMinimumDuration = async (startedAt: number, minimumMs: number) => {
  const remainingMs = Math.max(minimumMs - (performance.now() - startedAt), 0);
  if (remainingMs === 0) return;

  await new Promise((resolve) => window.setTimeout(resolve, remainingMs));
};

export const useMfaConnect = (
  method: CodeMfaStartMethod,
  { debounceMs = 0 }: UseMfaConnectOptions = {},
) => {
  const { location, setPostureError, setView } = useLocationCardContext();

  const [token, setToken] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(debounceMs > 0);
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
    const startedAt = performance.now();

    setIsStarting(true);

    (async () => {
      try {
        const { response, headers } = await startClientMfaSession({
          instance,
          location,
          method,
        });
        await waitForMinimumDuration(startedAt, debounceMs);
        setRequestHeaders(headers);
        setToken(response.token);
      } catch (err) {
        await waitForMinimumDuration(startedAt, debounceMs);
        if (handleMfaStartError({ err, location, setPostureError, setView })) {
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
