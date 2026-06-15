import { useMutation, useQuery } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { EdgeRequestHeaders } from '../../../edge-api/types';
import { api } from '../../../rust-api/api';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { LocationInfo } from '../../../rust-api/types';
import {
  CLIENT_MFA_ENDPOINT,
  type MfaStartMethod,
  shouldShowPostureError,
  startClientMfaSession,
} from '../api/startClientMfaSession';

type MfaFinishResponse = {
  preshared_key: string;
};

type MfaErrorResponse = {
  error: string;
};

type CodeMfaStartMethod = Extract<MfaStartMethod, 0 | 1>;

type UseMfaConnectOptions = {
  debounceMs?: number;
  onConnected?: () => void;
  onSessionExpired?: () => void;
  onPostureError?: (message: string) => void;
};

const waitForMinimumDuration = async (startedAt: number, minimumMs: number) => {
  const remainingMs = Math.max(minimumMs - (performance.now() - startedAt), 0);
  if (remainingMs === 0) return;

  await new Promise((resolve) => window.setTimeout(resolve, remainingMs));
};

export const useMfaConnect = (
  location: LocationInfo,
  method: CodeMfaStartMethod,
  {
    debounceMs = 0,
    onConnected,
    onSessionExpired,
    onPostureError,
  }: UseMfaConnectOptions = {},
) => {
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
      onConnected?.();
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
        if (shouldShowPostureError(err, location)) {
          onPostureError?.(err.message);
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
            onSessionExpired?.();
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
    [token, instance, requestHeaders, location, connectMutate, onSessionExpired],
  );

  return { token, isStarting, startError, verifyCode, isVerifying, verifyError };
};
