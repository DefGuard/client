import { useCallback, useEffect, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { CodeInput } from '../../../../../../../shared/components/CodeInput/CodeInput';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { IconKind } from '../../../../../../../shared/components/Icon';
import { IconButton } from '../../../../../../../shared/components/IconButton/IconButton';
import { IconButtonVariant } from '../../../../../../../shared/components/IconButton/types';
import { MfaStartMethod } from '../../../../../../../shared/components/LocationCard/api/startClientMfaSession';
import { useMfaConnect } from '../../../../../../../shared/components/LocationCard/hooks/useMfaConnect';
import type { LocationInfo } from '../../../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../../../shared/utils/isPresent';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';

const MIN_POSTURE_LOADER_MS = 500;

export const ConnectModalMfaEmail = () => {
  const [perviousView, location] = useConnectModal(
    useShallow((s) => [s.perviousView, s.location]),
  );

  const { verifyCode, isVerifying, verifyError, isStarting, startError } = useMfaConnect(
    location as LocationInfo,
    MfaStartMethod.Email,
    {
      debounceMs: location?.posture_check_required ? MIN_POSTURE_LOADER_MS : 0,
      onSessionExpired: () =>
        useConnectModal.getState().setView(perviousView ?? ConnectModalView.MfaSettings),
      onPostureError: () =>
        useConnectModal.getState().setView(ConnectModalView.PostureCheckFail),
    },
  );

  const [emailCode, setEmailCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleVerify = useCallback(() => {
    if (!isPresent(emailCode)) {
      setError('Enter code');
      return;
    }
    if (emailCode.length !== 6) {
      setError('6 digits are required');
      return;
    }
    verifyCode(emailCode);
  }, [emailCode, verifyCode]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect of code input
  useEffect(() => {
    setError(null);
  }, [emailCode, setError]);

  useEffect(() => {
    if (verifyError) setError(verifyError);
  }, [verifyError]);

  return (
    <div
      id="mfa-email-view"
      onKeyDown={(e) => {
        if (e.key === 'Enter') handleVerify();
      }}
    >
      <p className="view-description">
        Enter the 6-digit code sent to your email address.
      </p>
      <CodeInput
        length={6}
        value={emailCode}
        onChange={setEmailCode}
        error={startError ?? error}
      />
      <Controls>
        <IconButton
          variant={IconButtonVariant.BigSelected}
          icon={IconKind.ArrowBig}
          iconRotation="left"
          onClick={() =>
            useConnectModal
              .getState()
              .setView(perviousView ?? ConnectModalView.MfaSettings)
          }
        />
        <div className="right">
          <Button
            text="Verify"
            variant={ButtonVariant.Primary}
            onClick={handleVerify}
            loading={isStarting || isVerifying}
          />
        </div>
      </Controls>
    </div>
  );
};
