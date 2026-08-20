import { useQuery } from '@tanstack/react-query';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import {
  isConnectFailure,
  isInvalidCode,
  isMfaPostureError,
  isServiceUnavailable,
  isSessionExpired,
  mfaErrorMessage,
} from '../../../rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { LocationInfo, MfaMethod } from '../../../rust-api/types';

type CodeMfaMethod = typeof MfaMethod.Totp | typeof MfaMethod.Email;

type UseMfaConnectOptions = {
  debounceMs?: number;
  onStepPassed?: () => void;
  onConnected?: () => void;
  onSessionExpired?: () => void;
  onPostureError?: (message: string) => void;
  onServiceUnavailable?: () => void;
};

// TODO: delete this
const MOCK_STEP_TOKEN = 'mock-step-token';

const waitForMinimumDuration = async (startedAt: number, minimumMs: number) => {
  const remainingMs = Math.max(minimumMs - (performance.now() - startedAt), 0);
  if (remainingMs === 0) return;

  await new Promise((resolve) => window.setTimeout(resolve, remainingMs));
};

export const useMfaConnect = (
  location: LocationInfo,
  method: CodeMfaMethod,
  {
    debounceMs = 0,
    onStepPassed,
    onConnected,
    onSessionExpired,
    onPostureError,
    onServiceUnavailable,
  }: UseMfaConnectOptions = {},
) => {
  const [token, setToken] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(debounceMs > 0);
  const [startError, setStartError] = useState<string | null>(null);
  const [isVerifying, setIsVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState<string | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);

  const instance = instances?.find((i) => i.id === location.instance_id);

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
        // TODO(mock): drop this branch and always call mfaStart with the step's method
        if (onStepPassed) {
          await waitForMinimumDuration(startedAt, debounceMs);
          setToken(MOCK_STEP_TOKEN);
          return;
        }
        const info = await api.mfaStart(instance.id, location.id, method);
        await waitForMinimumDuration(startedAt, debounceMs);
        setToken(info.token);
      } catch (err) {
        void error(`MFA start failed: ${err}`);
        await waitForMinimumDuration(startedAt, debounceMs);
        if (isMfaPostureError(err, location)) {
          onPostureError?.(mfaErrorMessage(err));
          return;
        }
        if (isServiceUnavailable(err)) {
          onServiceUnavailable?.();
          return;
        }
        setStartError(mfaErrorMessage(err));
      } finally {
        setIsStarting(false);
      }
    })();
  }, [instance]);

  const verifyCode = useCallback(
    async (code: string) => {
      if (!token || !instance) return;

      setIsVerifying(true);
      setVerifyError(null);

      try {
        // TODO(mock): call mfaFinishCode here too and advance only on MfaAdvanced,
        // falling through to onConnected on MfaCompleted
        if (onStepPassed) {
          onStepPassed();
          return;
        }
        // mfaFinishCode completes MFA and brings up the connection in the
        // backend; the preshared key never reaches the frontend.
        await api.mfaFinishCode(instance.id, location.id, token, code);
        onConnected?.();
      } catch (err) {
        void error(`MFA verification failed: ${err}`);
        const message = mfaErrorMessage(err);
        if (isConnectFailure(message)) {
          setVerifyError('Failed to establish VPN connection');
        } else if (isInvalidCode(message)) {
          setVerifyError('Invalid code');
        } else if (isSessionExpired(message)) {
          onSessionExpired?.();
        } else {
          setVerifyError('Verification failed');
        }
      } finally {
        setIsVerifying(false);
      }
    },
    [token, instance, location, onStepPassed, onConnected, onSessionExpired],
  );

  return { token, isStarting, startError, verifyCode, isVerifying, verifyError };
};
