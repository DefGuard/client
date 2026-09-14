import { useMutation } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { CodeInput } from '../../../../../shared/components/CodeInput/CodeInput';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { api } from '../../../../../shared/rust-api/api';
import { MfaMethod } from '../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import { useMfaConfigErrorHandler } from '../../hooks/useMfaConfigErrorHandler';
import '../style.scss';

const CODE_LENGTH = 6;

interface Props {
  onCancel: () => void;
  /** The session outlived its deadline, so the whole flow has to start over. */
  onSessionExpired: () => void;
}

/** The fallback when no factor exists, or email picked from the configured code factors. */
export const ConfigureVerifyEmailStep = ({ onCancel, onSessionExpired }: Props) => {
  const sessionId = useConfigureMfaStore((s) => s.sessionId);

  const [code, setCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleApiError = useMfaConfigErrorHandler({
    context: 'Email MFA configuration verification failed',
    setError,
    onSessionExpired,
    fallback: 'Verification failed',
  });

  const { mutate: requestCode, isPending: isRequestingCode } = useMutation({
    mutationFn: async () => {
      if (!isPresent(sessionId)) {
        throw new Error('No MFA configuration session');
      }
      await api.mfaConfigSendCode(sessionId);
    },
    onError: handleApiError,
  });

  // A second call invalidates the code already sent, so fire once, StrictMode included.
  const requestedFor = useRef<string | null>(null);
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
    onError: handleApiError,
  });

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: discardMfaConfiguration,
    onSettled: onCancel,
  });

  const handleSubmit = useCallback(
    (pastedCode?: string) => {
      const toSubmit = (pastedCode ?? code)?.trim();
      if (toSubmit?.length !== CODE_LENGTH) {
        setError('Enter a valid code');
        return;
      }
      submitCode(toSubmit);
    },
    [code, submitCode],
  );

  // Only real input clears the error, CodeInput's own reset passes ''.
  const handleCodeChange = useCallback((value: string) => {
    setCode(value);
    if (value.length > 0) setError(null);
  }, []);

  return (
    <FullPage
      id="configure-verify-email-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Verification code sent to your email" />
      <p className="description">
        <span>{`We've sent a verification code to your email address.`}</span>
        <span>{`Please check your inbox and enter the 6-digit code from the email to continue.`}</span>
      </p>
      <div
        className="code-track"
        onKeyDown={(e) => {
          if (e.key === 'Enter') handleSubmit();
        }}
      >
        <CodeInput
          length={CODE_LENGTH}
          value={code}
          onChange={handleCodeChange}
          error={error}
          onSuccessPaste={(value) => {
            handleSubmit(value);
          }}
        />
      </div>
      <Controls>
        <Button
          text="Cancel"
          variant={ButtonVariant.Secondary}
          loading={isCancelling}
          onClick={() => {
            cancel();
          }}
        />
        <div className="right">
          <Button
            text="Verify"
            variant={ButtonVariant.Primary}
            loading={isRequestingCode || isSubmitting}
            onClick={() => {
              handleSubmit();
            }}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
