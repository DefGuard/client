import { useMutation } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
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

/** The code comes from an app the user already has, so there is nothing to request first. */
export const ConfigureVerifyTotpStep = ({ onCancel, onSessionExpired }: Props) => {
  const sessionId = useConfigureMfaStore((s) => s.sessionId);

  const [code, setCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleApiError = useMfaConfigErrorHandler({
    context: 'TOTP MFA configuration verification failed',
    setError,
    onSessionExpired,
    fallback: 'Verification failed',
  });

  const { mutate: submitCode, isPending: isSubmitting } = useMutation({
    mutationFn: async (value: string) => {
      if (!isPresent(sessionId)) {
        throw new Error('No MFA configuration session');
      }
      const result = await api.mfaConfigAuthorize(sessionId, MfaMethod.Totp, value);
      useConfigureMfaStore.getState().authorize(result);
    },
    onError: handleApiError,
  });

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: discardMfaConfiguration,
    onSettled: onCancel,
  });

  const isBusy = isSubmitting || isCancelling;

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

  // Only real input clears the error, CodeInput's own reset passes ''.
  const handleCodeChange = useCallback((value: string) => {
    setCode(value);
    if (value.length > 0) setError(null);
  }, []);

  return (
    <FullPage
      id="configure-verify-totp-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Verification code from your authentication app" />
      <p className="description">
        <span>{`Open the authentication app you use with Defguard.`}</span>
        <span>{`Enter the 6-digit code it currently displays to continue.`}</span>
      </p>
      <div className="code-track">
        <CodeInput
          length={CODE_LENGTH}
          value={code}
          onChange={handleCodeChange}
          error={error}
          onSubmit={() => {
            handleSubmit();
          }}
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
            loading={isSubmitting}
            disabled={isCancelling}
            onClick={() => {
              handleSubmit();
            }}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
