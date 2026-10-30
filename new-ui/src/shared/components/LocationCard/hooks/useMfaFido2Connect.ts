import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useMfaClientAttempt } from '../../../hooks/useMfaClientAttempt';
import { api } from '../../../rust-api/api';
import { fido2ShowsTouchPrompt } from '../../../rust-api/fido2';
import {
  isCancelled,
  isConnectFailure,
  isMfaPostureError,
  isSecurityKeyPinRejected,
  isServiceUnavailable,
  isTimeout,
  MfaFailureText,
  mfaErrorMessage,
} from '../../../rust-api/mfaError';
import type {
  LocationInfo,
  MfaErrorPayload,
  MfaFido2StepAdvancedPayload,
  MfaMethodValue,
} from '../../../rust-api/types';
import { TauriEvent } from '../../../rust-api/types';
import { isPresent } from '../../../utils/isPresent';

type Options = {
  stepPlan: MfaMethodValue[];
  mfaToken: string | null;
  setMfaToken: (token: string | null) => void;
  onConnected?: () => void;
  onStepAdvanced?: (nextStepIndex: number) => void;
  onPostureError?: (message: string) => void;
  onServiceUnavailable?: () => void;
  // replaces the other error callbacks and all non-input error state
  onFailure?: (err: unknown) => void;
};

/**
 * FIDO2 MFA. Starting verification kicks off a background task that gets the challenge from
 * Edge, signs it with the security key, and brings up the VPN. Results arrive as events.
 *
 * `pin` is null where the platform collects it in its own prompt.
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
    onFailure,
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

  const verify = useCallback(
    async (pin: string | null) => {
      const attempt = startAttempt();
      setIsVerifying(true);
      setIsAwaitingTouch(false);
      setVerifyError(null);

      const tryFinishAttempt = () => {
        if (!attempt.tryFinish()) return false;
        taskOutstandingRef.current = false;
        setIsVerifying(false);
        setIsAwaitingTouch(false);
        return true;
      };

      const fail = (err: unknown) => {
        if (isPresent(onFailure)) {
          onFailure(err);
        } else {
          setVerifyError(mfaErrorMessage(err));
        }
      };

      // Attach listeners before starting in case the task fails immediately.
      try {
        await Promise.all([
          attempt.ownListener(
            // A touch is progress, not the final result.
            listen(TauriEvent.MfaFido2Touch, () => {
              if (attempt.isLive()) setIsAwaitingTouch(fido2ShowsTouchPrompt());
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
              const retryable =
                isTimeout(event.payload.error) ||
                isServiceUnavailable(event.payload.error);
              if (!retryable) setMfaToken(null);
              if (isCancelled(event.payload.error)) return;
              void error(
                `FIDO2 MFA failed for location ${location.id}: ${event.payload.error}`,
              );
              const message = mfaErrorMessage(event.payload.error);
              const pinRejected = pin !== null && isSecurityKeyPinRejected(message);
              if (isPresent(onFailure) && !pinRejected) {
                onFailure(event.payload.error);
                return;
              }

              if (isMfaPostureError(event.payload.error, location)) {
                onPostureError?.(mfaErrorMessage(event.payload.error));
                return;
              }
              if (isServiceUnavailable(event.payload.error)) {
                onServiceUnavailable?.();
                return;
              }
              // the server error tells the user whether the key, PIN, or touch failed
              setVerifyError(
                isConnectFailure(message) ? MfaFailureText.ConnectFailure : message,
              );
            }),
          ),
        ]);
      } catch (err) {
        if (!tryFinishAttempt()) return;
        void error(`FIDO2 MFA listener setup failed for location ${location.id}: ${err}`);
        fail(err);
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
        void error(`FIDO2 MFA start failed for location ${location.id}: ${err}`);
        fail(err);
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
      onFailure,
    ],
  );

  return { verify, isVerifying, isAwaitingTouch, verifyError };
};
