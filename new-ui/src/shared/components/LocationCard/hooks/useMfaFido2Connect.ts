import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useMfaClientAttempt } from '../../../hooks/useMfaClientAttempt';
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

  const { startAttempt } = useMfaClientAttempt();

  /// Separate from the attempt's own task handle: the primitive cancels the task, but
  /// only this hook knows the token has to go with it.
  const taskOutstandingRef = useRef(false);

  /// The view state the primitive does not own.
  const resetPinView = useCallback(() => {
    setIsVerifying(false);
    setIsAwaitingTouch(false);
  }, []);

  // Give up the token on unmount, so a view left mid-touch does not keep one the
  // cancelled task would have consumed. Listeners and task are the primitive's job.
  useEffect(() => {
    return () => {
      if (taskOutstandingRef.current) {
        taskOutstandingRef.current = false;
        setMfaToken(null);
      }
    };
  }, [setMfaToken]);

  const verifyPin = useCallback(
    async (pin: string) => {
      const attempt = startAttempt();
      setIsVerifying(true);
      setIsAwaitingTouch(false);
      setVerifyError(null);

      /// Take the attempt's single outcome, and bring the view back with it.
      const tryFinishAttempt = () => {
        if (!attempt.tryFinish()) return false;
        taskOutstandingRef.current = false;
        resetPinView();
        return true;
      };

      // Listen before starting: a task that fails fast (no key plugged in)
      // would otherwise emit before the listeners are attached.
      try {
        await Promise.all([
          attempt.ownListener(
            // Progress, not an outcome, so it must not claim the attempt.
            listen(TauriEvent.MfaFido2Touch, () => {
              if (attempt.isLive()) setIsAwaitingTouch(true);
            }),
          ),
          attempt.ownListener(
            listen(TauriEvent.MfaFido2Complete, () => {
              if (!tryFinishAttempt()) return;
              onConnected?.();
            }),
          ),
          attempt.ownListener(
            listen<MfaFido2StepAdvancedPayload>(
              TauriEvent.MfaFido2StepAdvanced,
              (event) => {
                if (!tryFinishAttempt()) return;
                setMfaToken(event.payload.token);
                onStepAdvanced?.(event.payload.nextStep);
              },
            ),
          ),
          attempt.ownListener(
            listen<MfaErrorPayload>(TauriEvent.MfaFido2Error, (event) => {
              if (!tryFinishAttempt()) return;
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
        if (!tryFinishAttempt()) return;
        setMfaToken(null);
        void error(`FIDO2 MFA listener setup failed for location ${location.id}: ${err}`);
        setVerifyError(mfaErrorMessage(err));
        return;
      }

      if (!attempt.isLive()) return;

      try {
        const taskId = await api.mfaFido2Pin(
          location.instance_id,
          location.id,
          stepPlan,
          mfaToken,
          pin,
        );
        attempt.ownTask(taskId);
        if (!attempt.isLive()) return;
        taskOutstandingRef.current = true;
      } catch (err) {
        if (!tryFinishAttempt()) return;
        setMfaToken(null);
        void error(`FIDO2 MFA start failed for location ${location.id}: ${err}`);
        setVerifyError(mfaErrorMessage(err));
      }
    },
    [
      startAttempt,
      location,
      stepPlan,
      mfaToken,
      setMfaToken,
      resetPinView,
      onConnected,
      onStepAdvanced,
      onPostureError,
      onServiceUnavailable,
    ],
  );

  return { verifyPin, isVerifying, isAwaitingTouch, verifyError };
};
