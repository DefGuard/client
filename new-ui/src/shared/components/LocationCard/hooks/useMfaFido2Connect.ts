import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../../rust-api/api';
import {
  isConnectFailure,
  isMfaPostureError,
  isServiceUnavailable,
  mfaErrorMessage,
} from '../../../rust-api/mfaError';
import type {
  LocationInfo,
  MfaErrorPayload,
  MfaFido2StepAdvancedPayload,
  MfaMethodValue,
} from '../../../rust-api/types';
import { TauriEvent } from '../../../rust-api/types';

type Options = {
  stepPlan: MfaMethodValue[];
  mfaToken: string | null;
  setMfaToken: (token: string | null) => void;
  onConnected?: () => void;
  onStepAdvanced?: (nextStepIndex: number) => void;
  onPostureError?: (message: string) => void;
  onServiceUnavailable?: () => void;
};

/**
 * FIDO2 MFA. Submitting the PIN starts a backend task that asks Edge for the
 * challenge and the credential id, has the security key sign them, submits the
 * assertion and brings the connection up - so the outcome arrives as an event
 * rather than as the call's return value, like the other task-based methods.
 */
export const useMfaFido2Connect = (
  location: LocationInfo,
  {
    stepPlan,
    mfaToken,
    setMfaToken,
    onConnected,
    onStepAdvanced,
    onPostureError,
    onServiceUnavailable,
  }: Options,
) => {
  const [isVerifying, setIsVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState<string | null>(null);
  const [isAwaitingTouch, setIsAwaitingTouch] = useState(false);

  const operationRef = useRef(0);
  const taskIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const cleanupListeners = useCallback(() => {
    if (unlistenRef.current !== null) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  /// Every exit from a verification attempt lands here: listeners dropped, the
  /// task forgotten, the view back to accepting a PIN.
  const settle = useCallback(() => {
    cleanupListeners();
    taskIdRef.current = null;
    setIsVerifying(false);
    setIsAwaitingTouch(false);
  }, [cleanupListeners]);

  // Drop the listeners and abandon the key on unmount, so a view left mid-touch
  // does not connect behind the user's back.
  useEffect(() => {
    return () => {
      operationRef.current += 1;
      cleanupListeners();
      const taskId = taskIdRef.current;
      if (taskId) {
        void api.cancelMfa(taskId).catch(() => {});
        setMfaToken(null);
      }
    };
  }, [cleanupListeners, setMfaToken]);

  const verifyPin = useCallback(
    async (pin: string) => {
      const operation = ++operationRef.current;
      cleanupListeners();
      setIsVerifying(true);
      setIsAwaitingTouch(false);
      setVerifyError(null);

      // Listen before starting: a task that fails fast (no key plugged in)
      // would otherwise emit before the listeners are attached.
      const isCurrentOperation = () => operationRef.current === operation;
      const finishOperation = () => {
        if (!isCurrentOperation()) return false;
        operationRef.current += 1;
        settle();
        return true;
      };

      const registeredListeners: UnlistenFn[] = [];
      const cleanupRegisteredListeners = () => {
        for (const unlisten of registeredListeners.splice(0)) {
          unlisten();
        }
      };
      const registerListener = (listener: Promise<UnlistenFn>) =>
        listener.then((unlisten) => {
          if (isCurrentOperation()) {
            registeredListeners.push(unlisten);
          } else {
            unlisten();
          }
          return unlisten;
        });

      unlistenRef.current = cleanupRegisteredListeners;
      let listeners: [UnlistenFn, UnlistenFn, UnlistenFn, UnlistenFn];
      try {
        listeners = await Promise.all([
          registerListener(
            listen(TauriEvent.MfaFido2Touch, () => {
              if (isCurrentOperation()) setIsAwaitingTouch(true);
            }),
          ),
          registerListener(
            listen(TauriEvent.MfaFido2Complete, () => {
              if (!finishOperation()) return;
              onConnected?.();
            }),
          ),
          registerListener(
            listen<MfaFido2StepAdvancedPayload>(
              TauriEvent.MfaFido2StepAdvanced,
              (event) => {
                if (!finishOperation()) return;
                setMfaToken(event.payload.token);
                onStepAdvanced?.(event.payload.nextStep);
              },
            ),
          ),
          registerListener(
            listen<MfaErrorPayload>(TauriEvent.MfaFido2Error, (event) => {
              if (!finishOperation()) return;
              setMfaToken(null);
              void error(
                `FIDO2 MFA failed for location ${location.id}: ${event.payload.error}`,
              );

              if (isMfaPostureError(event.payload.error, location)) {
                onPostureError?.(mfaErrorMessage(event.payload.error));
                return;
              }
              if (isServiceUnavailable(event.payload.error)) {
                onServiceUnavailable?.();
                return;
              }
              const message = mfaErrorMessage(event.payload.error);
              // The backend's messages name what actually went wrong (no key, wrong
              // PIN, no touch), so they are worth showing as they are.
              setVerifyError(
                isConnectFailure(message)
                  ? 'Failed to establish VPN connection'
                  : message,
              );
            }),
          ),
        ]);
      } catch (err) {
        const isCurrent = finishOperation();
        cleanupRegisteredListeners();
        if (!isCurrent) return;
        setMfaToken(null);
        void error(`FIDO2 MFA listener setup failed for location ${location.id}: ${err}`);
        setVerifyError(mfaErrorMessage(err));
        return;
      }

      if (!isCurrentOperation()) {
        cleanupRegisteredListeners();
        return;
      }

      const [touchUnlisten, completeUnlisten, stepAdvancedUnlisten, errorUnlisten] =
        listeners;
      unlistenRef.current = () => {
        touchUnlisten();
        completeUnlisten();
        stepAdvancedUnlisten();
        errorUnlisten();
      };

      try {
        const taskId = await api.mfaFido2Pin(
          location.instance_id,
          location.id,
          stepPlan,
          mfaToken,
          pin,
        );
        if (!isCurrentOperation()) {
          void api.cancelMfa(taskId).catch(() => {});
          return;
        }
        taskIdRef.current = taskId;
      } catch (err) {
        if (!finishOperation()) return;
        setMfaToken(null);
        void error(`FIDO2 MFA start failed for location ${location.id}: ${err}`);
        setVerifyError(mfaErrorMessage(err));
      }
    },
    [
      location,
      stepPlan,
      mfaToken,
      setMfaToken,
      cleanupListeners,
      settle,
      onConnected,
      onStepAdvanced,
      onPostureError,
      onServiceUnavailable,
    ],
  );

  return { verifyPin, isVerifying, isAwaitingTouch, verifyError };
};
