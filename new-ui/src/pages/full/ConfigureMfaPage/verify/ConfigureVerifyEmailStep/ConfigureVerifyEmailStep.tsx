import { useMutation } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { IconKind } from '../../../../../shared/components/Icon';
import { ResendEmailButton } from '../../../../../shared/components/ResendEmailButton/ResendEmailButton';
import { useResendCooldown } from '../../../../../shared/hooks/useResendCooldown';
import { api } from '../../../../../shared/rust-api/api';
import { MfaMethod } from '../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { MfaCodeVerification } from '../../components/MfaCodeVerification/MfaCodeVerification';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';
import { useLeaveVerification } from '../../hooks/useLeaveVerification';
import { useMfaConfigErrorHandler } from '../../hooks/useMfaConfigErrorHandler';

const CODE_LENGTH = 6;

interface Props {
  onCancel: () => void;
  /** The session outlived its deadline, so the whole flow has to start over. */
  onSessionExpired: () => void;
}

/** The fallback when no factor exists, or email picked from the configured code factors. */
export const ConfigureVerifyEmailStep = ({ onCancel, onSessionExpired }: Props) => {
  const sessionId = useConfigureMfaStore((s) => s.sessionId);
  const { leaveText, leave, isLeaving } = useLeaveVerification(onCancel);

  const [code, setCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const cooldown = useResendCooldown();

  const handleApiError = useMfaConfigErrorHandler({
    context: 'Email MFA configuration verification failed',
    setError,
    onSessionExpired,
    fallback: 'Verification failed',
  });

  // A second call invalidates the code already sent, so fire once, StrictMode included.
  const requestedFor = useRef<string | null>(null);

  const { mutate: requestCode, isPending: isRequestingCode } = useMutation({
    mutationFn: async () => {
      if (!isPresent(sessionId)) {
        throw new Error('No MFA configuration session');
      }
      await api.mfaConfigSendCode(sessionId);
    },
    onSuccess: () => {
      cooldown.restart();
    },
    onError: (err) => {
      cooldown.reset();
      requestedFor.current = null;
      handleApiError(err, () => {
        requestedFor.current = sessionId;
        requestCode();
      });
    },
  });

  useEffect(() => {
    if (!isPresent(sessionId)) return;
    if (requestedFor.current === sessionId) return;
    requestedFor.current = sessionId;
    requestCode();
  }, [sessionId, requestCode]);

  const { mutate: submitCode, isPending: isSubmitting } = useMutation({
    mutationFn: async (value: string) => {
      if (!isPresent(sessionId)) {
        throw new Error('No MFA configuration session');
      }
      const result = await api.mfaConfigAuthorize(sessionId, MfaMethod.Email, value);
      useConfigureMfaStore.getState().authorize(result);
    },
    onError: (err) => handleApiError(err),
  });

  const isBusy = isRequestingCode || isSubmitting;

  const handleSubmit = useCallback(
    (pastedCode?: string) => {
      if (isBusy) return;
      const toSubmit = (pastedCode ?? code)?.trim();
      if (toSubmit?.length !== CODE_LENGTH) {
        setError('Enter a valid code');
        return;
      }
      submitCode(toSubmit);
    },
    [code, isBusy, submitCode],
  );

  const handleResend = useCallback(() => {
    if (isBusy || !isPresent(sessionId)) return;
    requestedFor.current = sessionId;
    setCode(null);
    setError(null);
    requestCode();
  }, [isBusy, requestCode, sessionId]);

  // Only real input clears the error, CodeInput's own reset passes ''.
  const handleCodeChange = useCallback((value: string) => {
    setCode(value);
    if (value.length > 0) setError(null);
  }, []);

  return (
    <MfaCodeVerification
      id="configure-verify-email-step"
      icon={IconKind.Sent}
      title="Email verification code"
      description={[
        `We've sent a verification code to your email address.`,
        `Enter the 6-digit code to continue setting up MFA.`,
      ]}
      code={code}
      error={error}
      onCodeChange={handleCodeChange}
      onSubmit={handleSubmit}
      codeLoading={isBusy}
      leaveText={leaveText}
      onLeave={leave}
      isLeaving={isLeaving}
      leaveDisabled={isSubmitting}
      submitLoading={isSubmitting}
      submitDisabled={isRequestingCode || isLeaving}
      extraAction={
        <ResendEmailButton
          sent={cooldown.sent}
          loading={isRequestingCode}
          disabled={isSubmitting || isLeaving}
          onClick={handleResend}
        />
      }
    />
  );
};
