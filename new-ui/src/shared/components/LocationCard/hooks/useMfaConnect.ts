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
import type { LocationInfo, MfaMethod, MfaMethodValue } from '../../../rust-api/types';
import { isPresent } from '../../../utils/isPresent';

type CodeMfaMethod = typeof MfaMethod.Totp | typeof MfaMethod.Email;

type UseMfaConnectOptions = {
  debounceMs?: number;
  stepPlan: MfaMethodValue[];
  mfaToken: string | null;
  setMfaToken: (token: string) => void;
  onStepAdvanced: (nextStepIndex: number) => void;
  onConnected?: () => void;
  onSessionExpired?: () => void;
  onPostureError?: (message: string) => void;
  onServiceUnavailable?: () => void;
};

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
    stepPlan,
    mfaToken,
    setMfaToken,
    onStepAdvanced,
    onConnected,
    onSessionExpired,
    onPostureError,
    onServiceUnavailable,
  }: UseMfaConnectOptions,
) => {
  const [token, setToken] = useState<string | null>(mfaToken);
  const [stepAttemptId, setStepAttemptId] = useState<string | null>(null);
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
        const session = await api.startMfaStep(
          instance.id,
          location.id,
          method,
          stepPlan,
          mfaToken,
        );
        await waitForMinimumDuration(startedAt, debounceMs);
        setToken(session.token);
        setStepAttemptId(session.stepAttemptId);
        setMfaToken(session.token);
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
        const nextStepIndex = await api.mfaFinishCode(
          instance.id,
          location.id,
          token,
          code,
          stepAttemptId,
        );
        if (isPresent(nextStepIndex)) {
          onStepAdvanced(nextStepIndex);
          return;
        }
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
    [
      token,
      stepAttemptId,
      instance,
      location,
      onStepAdvanced,
      onConnected,
      onSessionExpired,
    ],
  );

  return { token, isStarting, startError, verifyCode, isVerifying, verifyError };
};
