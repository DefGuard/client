import { encode } from '@stablelib/base64';
import { useQuery } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import {
  isConnectFailure,
  isMfaPostureError,
  isServiceUnavailable,
  isTimeout,
  mfaErrorMessage,
} from '../../../rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type {
  LocationInfo,
  MfaErrorPayload,
  MfaMethodValue,
  MfaStepAdvancedPayload,
} from '../../../rust-api/types';
import { MfaMethod, TauriEvent } from '../../../rust-api/types';
import { isPresent } from '../../../utils/isPresent';

type TokenData = {
  token: string;
  challenge: string;
  stepAttemptId: string | null;
};

type Options = {
  stepPlan: MfaMethodValue[];
  mfaToken: string | null;
  setMfaToken: (token: string | null) => void;
  onStepAdvanced: (nextStepIndex: number) => void;
  onConnected?: () => void;
  onPostureError?: (message?: string) => void;
  onServiceUnavailable?: () => void;
};

export const useMfaMobileConnect = (
  location: LocationInfo,
  {
    stepPlan,
    mfaToken,
    setMfaToken,
    onStepAdvanced,
    onConnected,
    onPostureError,
    onServiceUnavailable,
  }: Options,
) => {
  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location.instance_id);

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [tokenData, setTokenData] = useState<TokenData | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);

  const taskIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const attemptGenerationRef = useRef(0);
  const onConnectedRef = useRef(onConnected);
  const onStepAdvancedRef = useRef(onStepAdvanced);
  const setMfaTokenRef = useRef(setMfaToken);
  const instanceId = instance?.id;

  onConnectedRef.current = onConnected;
  onStepAdvancedRef.current = onStepAdvanced;
  setMfaTokenRef.current = setMfaToken;

  const cleanupListeners = useCallback(() => {
    if (unlistenRef.current !== null) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      cleanupListeners();
      const taskId = taskIdRef.current;
      if (taskId) {
        void api.cancelMfa(taskId).catch(() => {});
        setMfaTokenRef.current(null);
      }
    };
  }, [cleanupListeners]);

  // Connect WebSocket via Rust when tokenData is available
  useEffect(() => {
    if (!tokenData || instanceId === undefined) return;

    let cancelled = false;
    const attemptGeneration = attemptGenerationRef.current;
    const isCurrentAttempt = () =>
      !cancelled && attemptGenerationRef.current === attemptGeneration;
    cleanupListeners();
    setIsConnecting(true);
    setConnectionError(null);

    (async () => {
      try {
        const taskId = await api.mfaConnectMobileApprove(
          instanceId,
          location.id,
          tokenData.token,
        );
        if (!isCurrentAttempt()) {
          void api.cancelMfa(taskId).catch(() => {});
          return;
        }
        taskIdRef.current = taskId;

        // The backend brings up the connection itself; completion means connected.
        const completeUnlisten = await listen(TauriEvent.MfaMobileComplete, () => {
          if (!isCurrentAttempt()) return;
          cleanupListeners();
          taskIdRef.current = null;
          setIsConnecting(false);
          onConnectedRef.current?.();
        });
        if (!isCurrentAttempt()) {
          completeUnlisten();
          return;
        }

        const stepAdvancedUnlisten = await listen<MfaStepAdvancedPayload>(
          TauriEvent.MfaMobileStepAdvanced,
          (event) => {
            if (!isCurrentAttempt()) return;
            cleanupListeners();
            taskIdRef.current = null;
            setIsConnecting(false);
            onStepAdvancedRef.current(event.payload.nextStep);
          },
        );
        if (!isCurrentAttempt()) {
          completeUnlisten();
          stepAdvancedUnlisten();
          return;
        }

        const errorUnlisten = await listen<MfaErrorPayload>(
          TauriEvent.MfaMobileError,
          (event) => {
            if (!isCurrentAttempt()) return;
            cleanupListeners();
            taskIdRef.current = null;
            setIsConnecting(false);
            void error(
              `Mobile MFA failed for location ${location.id}: ${event.payload.error}`,
            );
            const message = mfaErrorMessage(event.payload.error);
            const retryable =
              isTimeout(event.payload.error) || isServiceUnavailable(event.payload.error);
            setTokenData(null);
            if (!retryable) setMfaTokenRef.current(null);
            setConnectionError(
              isConnectFailure(message)
                ? 'Failed to establish VPN connection'
                : 'Connection error. Please try again.',
            );
          },
        );
        if (!isCurrentAttempt()) {
          completeUnlisten();
          stepAdvancedUnlisten();
          errorUnlisten();
          return;
        }

        unlistenRef.current = () => {
          completeUnlisten();
          stepAdvancedUnlisten();
          errorUnlisten();
        };
      } catch (e) {
        if (isCurrentAttempt()) {
          setIsConnecting(false);
          setTokenData(null);
          const taskId = taskIdRef.current;
          if (taskId) {
            taskIdRef.current = null;
            void api.cancelMfa(taskId).catch(() => {});
            setMfaTokenRef.current(null);
          } else if (!isTimeout(e) && !isServiceUnavailable(e)) {
            setMfaTokenRef.current(null);
          }
          setConnectionError('Failed to start mobile approval. Please try again.');
          void error(`Mobile MFA connect failed for location ${location.id}: ${e}`);
        }
      }
    })();

    return () => {
      cancelled = true;
      cleanupListeners();
      setIsConnecting(false);
    };
  }, [tokenData, instanceId, location.id, cleanupListeners]);

  const qrValue = useMemo(() => {
    if (!tokenData || !instance) return null;
    const json = JSON.stringify({
      token: tokenData.token,
      challenge: tokenData.challenge,
      ...(isPresent(tokenData.stepAttemptId)
        ? { step_attempt_id: tokenData.stepAttemptId }
        : {}),
      instance_id: instance.uuid,
    });
    return encode(new TextEncoder().encode(json));
  }, [tokenData, instance]);

  const start = useCallback(async () => {
    if (!instance) {
      setStartError('Instance not found');
      return;
    }

    const replacingAttempt = taskIdRef.current !== null || tokenData !== null;
    const attemptGeneration = ++attemptGenerationRef.current;
    const taskId = taskIdRef.current;
    if (replacingAttempt) cleanupListeners();
    if (taskId) {
      taskIdRef.current = null;
      void api.cancelMfa(taskId).catch(() => {});
    }
    if (replacingAttempt) setMfaToken(null);

    setIsStarting(true);
    setStartError(null);
    setConnectionError(null);
    setTokenData(null);

    try {
      const session = await api.mfaBeginStep(
        instance.id,
        location.id,
        MfaMethod.MobileApprove,
        stepPlan,
        replacingAttempt ? null : mfaToken,
      );
      if (attemptGenerationRef.current !== attemptGeneration) return;
      setMfaToken(session.token);

      if (!isPresent(session.challenge)) {
        setStartError('Unsupported response from proxy');
        return;
      }

      setTokenData({
        token: session.token,
        challenge: session.challenge,
        stepAttemptId: session.stepAttemptId,
      });
    } catch (e) {
      if (attemptGenerationRef.current !== attemptGeneration) return;
      void error(`Mobile MFA start failed for location ${location.id}: ${e}`);
      const retryable = isTimeout(e) || isServiceUnavailable(e);
      if (!retryable) setMfaToken(null);
      if (isMfaPostureError(e, location)) {
        onPostureError?.(mfaErrorMessage(e));
        return;
      }
      if (isServiceUnavailable(e)) {
        onServiceUnavailable?.();
        return;
      }
      setStartError(mfaErrorMessage(e));
    } finally {
      if (attemptGenerationRef.current === attemptGeneration) setIsStarting(false);
    }
  }, [
    instance,
    location,
    stepPlan,
    mfaToken,
    tokenData,
    cleanupListeners,
    setMfaToken,
    onPostureError,
    onServiceUnavailable,
  ]);

  const reset = useCallback(() => {
    attemptGenerationRef.current += 1;
    cleanupListeners();
    const taskId = taskIdRef.current;
    if (taskId) {
      void api.cancelMfa(taskId).catch(() => {});
      taskIdRef.current = null;
      setMfaToken(null);
    }
    setTokenData(null);
    setIsStarting(false);
    setStartError(null);
    setIsConnecting(false);
    setConnectionError(null);
  }, [cleanupListeners, setMfaToken]);

  return { start, isStarting, startError, qrValue, isConnecting, connectionError, reset };
};
