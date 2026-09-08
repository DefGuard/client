import { useQuery } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { api } from '../../../../../../shared/rust-api/api';
import {
  isAttemptLimit,
  isConnectFailure,
  isMfaPostureError,
  isServiceUnavailable,
  isSessionExpired,
  isStaleAttempt,
  isTimeout,
  mfaErrorMessage,
} from '../../../../../../shared/rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../../../../shared/rust-api/query';
import type {
  MfaErrorPayload,
  MfaStepAdvancedPayload,
} from '../../../../../../shared/rust-api/types';
import { MfaMethod, TauriEvent } from '../../../../../../shared/rust-api/types';
import { useConnectModal } from './useConnectModal';

type Options = {
  onPostureError?: (msg: string) => void;
  onSessionExpired?: () => void;
  onServiceUnavailable?: () => void;
};

export const useConnectModalMfaOidc = ({
  onPostureError,
  onSessionExpired,
  onServiceUnavailable,
}: Options = {}) => {
  const [location, stepPlan, mfaToken, setMfaToken, goToStep] = useConnectModal(
    useShallow((s) => [s.location, s.stepPlan, s.mfaToken, s.setMfaToken, s.goToStep]),
  );

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [pollError, setPollError] = useState<string | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location?.instance_id);

  const taskIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const operationRef = useRef(0);

  const cleanup = useCallback(() => {
    if (unlistenRef.current !== null) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  const cancelTask = useCallback((taskId: string) => {
    void api.cancelMfa(taskId).catch(() => {});
  }, []);

  const cancelCurrentTask = useCallback(() => {
    const taskId = taskIdRef.current;
    taskIdRef.current = null;
    if (taskId) {
      cancelTask(taskId);
    }
  }, [cancelTask]);

  useEffect(() => {
    return () => {
      operationRef.current += 1;
      cleanup();
      cancelCurrentTask();
    };
  }, [cancelCurrentTask, cleanup]);

  const start = useCallback(async () => {
    if (!instance || !location) {
      setStartError('Instance not found');
      return;
    }

    const operation = ++operationRef.current;
    setIsStarting(true);
    setStartError(null);
    setPollError(null);
    cleanup();
    cancelCurrentTask();

    try {
      const session = await api.mfaBeginStep(
        instance.id,
        location.id,
        MfaMethod.Oidc,
        stepPlan,
        mfaToken,
      );
      if (operationRef.current !== operation) return;

      const openIdUrl = new URL(
        'openid/mfa',
        instance.proxy_url.endsWith('/') ? instance.proxy_url : `${instance.proxy_url}/`,
      );
      openIdUrl.searchParams.set('token', session.token);
      if (session.step_attempt_id) {
        openIdUrl.searchParams.set('step_attempt_id', session.step_attempt_id);
      }
      await api.openLink(openIdUrl.toString());
      if (operationRef.current !== operation) return;
      setMfaToken(session.token);

      setIsStarting(false);
      setIsPolling(true);

      const taskId = await api.mfaPollOpenId(
        instance.id,
        location.id,
        session.token,
        session.step_attempt_id,
      );
      if (operationRef.current !== operation) {
        cancelTask(taskId);
        return;
      }
      taskIdRef.current = taskId;

      const unlistenFns: UnlistenFn[] = [];
      const removeListeners = () => {
        unlistenFns.splice(0).forEach((unlisten) => {
          unlisten();
        });
      };
      const cleanupStaleListeners = () => {
        removeListeners();
        if (unlistenRef.current === removeListeners) {
          unlistenRef.current = null;
        }
      };
      unlistenRef.current = removeListeners;

      const finishOperation = () => {
        if (operationRef.current !== operation) return false;
        operationRef.current += 1;
        taskIdRef.current = null;
        cleanup();
        return true;
      };

      // The backend brings up the connection itself; completion means connected.
      const completeUnlisten = await listen(TauriEvent.MfaOpenIdComplete, () => {
        if (!finishOperation()) return;
        setIsPolling(false);
      });
      unlistenFns.push(completeUnlisten);
      if (operationRef.current !== operation) {
        cleanupStaleListeners();
        return;
      }

      const stepAdvancedUnlisten = await listen<MfaStepAdvancedPayload>(
        TauriEvent.MfaOpenIdStepAdvanced,
        (event) => {
          if (!finishOperation()) return;
          setIsPolling(false);
          goToStep(event.payload.next_step);
        },
      );
      unlistenFns.push(stepAdvancedUnlisten);

      if (operationRef.current !== operation) {
        cleanupStaleListeners();
        return;
      }

      const errorUnlisten = await listen<MfaErrorPayload>(
        TauriEvent.MfaOpenIdError,
        (event) => {
          if (!finishOperation()) return;
          setIsPolling(false);
          error('OIDC MFA failed');
          const message = mfaErrorMessage(event.payload.error);
          if (isAttemptLimit(event.payload.error)) {
            setPollError(message);
          } else if (isStaleAttempt(message)) {
            setPollError('This MFA attempt is no longer valid. Please try again.');
          } else if (isTimeout(event.payload.error)) {
            setPollError('Authentication timed out. Please try again.');
          } else if (isConnectFailure(message)) {
            setPollError('Failed to establish VPN connection');
          } else if (isSessionExpired(message)) {
            onSessionExpired?.();
          } else {
            setPollError('Authentication failed. Please try again.');
          }
        },
      );
      unlistenFns.push(errorUnlisten);

      if (operationRef.current !== operation) {
        cleanupStaleListeners();
        return;
      }
    } catch (e) {
      if (operationRef.current !== operation) return;
      cleanup();
      cancelCurrentTask();
      void error('OIDC MFA start failed');
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
      if (operationRef.current === operation) {
        setIsStarting(false);
      }
    }
  }, [
    instance,
    location,
    stepPlan,
    mfaToken,
    setMfaToken,
    goToStep,
    cancelCurrentTask,
    cancelTask,
    cleanup,
    onPostureError,
    onSessionExpired,
    onServiceUnavailable,
  ]);

  return { start, isStarting, startError, isPolling, pollError };
};
