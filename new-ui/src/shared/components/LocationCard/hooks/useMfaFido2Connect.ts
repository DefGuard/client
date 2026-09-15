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
 * FIDO2 MFA. The PIN starts a background task that gets the challenge from Edge,
 * signs it with the security key, and brings up the VPN. Results arrive as events.
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

  // The attempt hook cancels the task; this hook must release its token too.
  const taskOutstandingRef = useRef(false);

  // Release the token on unmount so an abandoned view does not keep it for a cancelled task.
  // The attempt hook handles task and listener cleanup.
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

      // Finish this attempt and update the view.
      const tryFinishAttempt = () => {
        if (!attempt.tryFinish()) return false;
        taskOutstandingRef.current = false;
        setIsVerifying(false);
        setIsAwaitingTouch(false);
        return true;
      };

      // Attach listeners before starting in case the task fails immediately.
      try {
        await Promise.all([
          attempt.ownListener(
            // A touch is progress, not the final result.
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
              // Show the server error; it tells the user whether the key, PIN, or touch failed.
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
      onConnected,
      onStepAdvanced,
      onPostureError,
      onServiceUnavailable,
    ],
  );

  return { verifyPin, isVerifying, isAwaitingTouch, verifyError };
};
