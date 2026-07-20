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
import type { LocationInfo } from '../../../rust-api/types';
import { MfaMethod } from '../../../rust-api/types';

/** MFA method identifiers matching the proxy proto enum (numeric).
 *  Used by TOTP/email view callers that pass the method to useMfaConnect. */
export const MfaStartMethod = {
  Totp: 0,
  Email: 1,
  Oidc: 2,
  MobileApprove: 4,
} as const;

export type MfaStartMethod = (typeof MfaStartMethod)[keyof typeof MfaStartMethod];

type CodeMfaStartMethod = Extract<MfaStartMethod, 0 | 1>;

const MFA_METHOD_MAP: Record<number, string> = {
  [MfaStartMethod.Totp]: MfaMethod.Totp,
  [MfaStartMethod.Email]: MfaMethod.Email,
};

type UseMfaConnectOptions = {
  debounceMs?: number;
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
  method: CodeMfaStartMethod,
  {
    debounceMs = 0,
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

    const methodString = MFA_METHOD_MAP[method];
    if (!methodString) {
      setStartError('Unsupported MFA method');
      setIsStarting(false);
      return;
    }

    (async () => {
      try {
        const info = await api.mfaStart(instance.id, location.id, methodString);
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
    [token, instance, location, onConnected, onSessionExpired],
  );

  return { token, isStarting, startError, verifyCode, isVerifying, verifyError };
};
