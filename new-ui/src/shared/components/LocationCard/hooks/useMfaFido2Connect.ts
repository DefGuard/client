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
  MfaMethodValue,
} from '../../../rust-api/types';
import { TauriEvent } from '../../../rust-api/types';

type Options = {
  stepPlan: MfaMethodValue[];
  mfaToken: string | null;
  onConnected?: () => void;
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
  { stepPlan, mfaToken, onConnected, onPostureError, onServiceUnavailable }: Options,
) => {
  const [isVerifying, setIsVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState<string | null>(null);
  const [isAwaitingTouch, setIsAwaitingTouch] = useState(false);

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
      cleanupListeners();
      const taskId = taskIdRef.current;
      if (taskId) {
        void api.cancelMfa(taskId).catch(() => {});
      }
    };
  }, [cleanupListeners]);

  const verifyPin = useCallback(
    async (pin: string) => {
      cleanupListeners();
      setIsVerifying(true);
      setIsAwaitingTouch(false);
      setVerifyError(null);

      // Listen before starting: a task that fails fast (no key plugged in)
      // would otherwise emit before the listeners are attached.
      const [touchUnlisten, completeUnlisten, errorUnlisten] = await Promise.all([
        listen(TauriEvent.MfaFido2Touch, () => {
          setIsAwaitingTouch(true);
        }),
        listen(TauriEvent.MfaFido2Complete, () => {
          settle();
          onConnected?.();
        }),
        listen<MfaErrorPayload>(TauriEvent.MfaFido2Error, (event) => {
          settle();
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
            isConnectFailure(message) ? 'Failed to establish VPN connection' : message,
          );
        }),
      ]);
      unlistenRef.current = () => {
        touchUnlisten();
        completeUnlisten();
        errorUnlisten();
      };

      try {
        taskIdRef.current = await api.mfaFido2Pin(
          location.instance_id,
          location.id,
          stepPlan,
          mfaToken,
          pin,
        );
      } catch (err) {
        settle();
        void error(`FIDO2 MFA start failed for location ${location.id}: ${err}`);
        setVerifyError(mfaErrorMessage(err));
      }
    },
    [
      location,
      stepPlan,
      mfaToken,
      cleanupListeners,
      settle,
      onConnected,
      onPostureError,
      onServiceUnavailable,
    ],
  );

  return { verifyPin, isVerifying, isAwaitingTouch, verifyError };
};
