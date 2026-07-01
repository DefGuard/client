import { useCallback, useEffect, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { CodeInput } from '../../../../../../../shared/components/CodeInput/CodeInput';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { MfaStartMethod } from '../../../../../../../shared/components/LocationCard/api/startClientMfaSession';
import { useMfaConnect } from '../../../../../../../shared/components/LocationCard/hooks/useMfaConnect';
import type { LocationInfo } from '../../../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../../../shared/utils/isPresent';
import { ConnectModalPostureCheckLoading } from '../../components/ConnectModalPostureCheckLoading/ConnectModalPostureCheckLoading';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';

const MIN_POSTURE_LOADER_MS = 500;

export const ConnectModalMfaTotp = () => {
  const [perviousView, location] = useConnectModal(
    useShallow((s) => [s.perviousView, s.location]),
  );

  const { verifyCode, isVerifying, verifyError, isStarting, startError } = useMfaConnect(
    location as LocationInfo,
    MfaStartMethod.Totp,
    {
      debounceMs: location?.posture_check_required ? MIN_POSTURE_LOADER_MS : 0,
      onSessionExpired: () =>
        useConnectModal.getState().setView(perviousView ?? ConnectModalView.MfaSettings),
      onPostureError: (err) => {
        useConnectModal.setState({ postureError: err });
        useConnectModal.getState().setView(ConnectModalView.PostureCheckFail);
      },
    },
  );

  const [totpCode, setTotpCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleVerify = useCallback(
    (initCode?: string | null) => {
      const codeToVerify = initCode ?? totpCode;
      if (!isPresent(codeToVerify)) {
        setError('Enter code');
        return;
      }
      if (codeToVerify.replaceAll(' ', '').length !== 6) {
        setError('6 digits are required');
        return;
      }
      verifyCode(codeToVerify);
    },
    [totpCode, verifyCode],
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect of code input
  useEffect(() => {
    setError(null);
  }, [totpCode, setError]);

  useEffect(() => {
    if (verifyError) setError(verifyError);
  }, [verifyError]);

  if (isStarting && location?.posture_check_required && !startError) {
    return <ConnectModalPostureCheckLoading />;
  }

  return (
    <div
      id="mfa-totp-view"
      onKeyDown={(e) => {
        if (e.key === 'Enter') handleVerify();
      }}
    >
      <p className="view-description">
        Paste the code from your Authenticator Application.
      </p>
      <CodeInput
        length={6}
        value={totpCode}
        onChange={(val) => setTotpCode(val)}
        error={startError ?? error}
        onSuccessPaste={(value) => {
          handleVerify(value);
        }}
      />
      <Controls>
        <Button
          variant={ButtonVariant.Secondary}
          text="Use different MFA"
          onClick={() => {
            useConnectModal.getState().setView(ConnectModalView.MfaSettings);
          }}
        />
        <div className="right">
          <Button
            text="Verify"
            variant={ButtonVariant.Primary}
            onClick={() => handleVerify()}
            loading={isVerifying}
            disabled={isStarting}
          />
        </div>
      </Controls>
    </div>
  );
};
