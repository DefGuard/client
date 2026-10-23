import { useMutation } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { CodeInput } from '../../../../../shared/components/CodeInput/CodeInput';
import { IconKind } from '../../../../../shared/components/Icon';
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

/** The code comes from an app the user already has, so there is nothing to request first. */
export const ConfigureVerifyTotpStep = ({ onCancel, onSessionExpired }: Props) => {
  const sessionId = useConfigureMfaStore((s) => s.sessionId);
  const { leaveText, leave, isLeaving } = useLeaveVerification(onCancel);

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
    onError: (err) => handleApiError(err),
  });

  const handleSubmit = useCallback(
    (pastedCode?: string) => {
      if (isSubmitting) return;
      const toSubmit = (pastedCode ?? code)?.trim();
      if (toSubmit?.length !== CODE_LENGTH) {
        setError('Enter a valid code');
        return;
      }
      submitCode(toSubmit);
    },
    [code, isSubmitting, submitCode],
  );

  // Only real input clears the error, CodeInput's own reset passes ''.
  const handleCodeChange = useCallback((value: string) => {
    setCode(value);
    if (value.length > 0) setError(null);
  }, []);

  return (
    <MfaCodeVerification
      id="configure-verify-totp-step"
      icon={IconKind.MobileLock}
      title="Security code from your authenticator app."
      description={[`Enter the 6-digit code from your Authenticator app to continue.`]}
      onSubmit={handleSubmit}
      leaveText={leaveText}
      onLeave={leave}
      isLeaving={isLeaving}
      leaveDisabled={isSubmitting}
      submitLoading={isSubmitting}
      submitDisabled={isLeaving}
    >
      <CodeInput
        length={CODE_LENGTH}
        value={code}
        onChange={handleCodeChange}
        error={error}
        onSubmit={handleSubmit}
        loading={isSubmitting}
      />
    </MfaCodeVerification>
  );
};
