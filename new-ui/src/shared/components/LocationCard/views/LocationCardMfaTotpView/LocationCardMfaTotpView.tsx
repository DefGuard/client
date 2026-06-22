import { useCallback, useEffect, useState } from 'react';
import { ThemeSpacing } from '../../../../types';
import { isPresent } from '../../../../utils/isPresent';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { CodeInput } from '../../../CodeInput/CodeInput';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { MfaStartMethod } from '../../api/startClientMfaSession';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { useMfaConnect } from '../../hooks/useMfaConnect';
import { LocationCardMfaStartLoader } from '../LocationCardMfaStartLoader/LocationCardMfaStartLoader';

const MIN_POSTURE_LOADER_MS = 500;

export const LocationCardMfaTotpView = () => {
  const { setView, location, setPostureError } = useLocationCardContext();
  const { verifyCode, isVerifying, verifyError, isStarting, startError } = useMfaConnect(
    location,
    MfaStartMethod.Totp,
    {
      debounceMs: location.posture_check_required ? MIN_POSTURE_LOADER_MS : 0,
      onConnected: () => setView(LocationCardViews.Connected),
      onSessionExpired: () => setView(LocationCardViews.Default),
      onPostureError: (msg) => {
        setPostureError(msg);
        setView(LocationCardViews.PostureCheckFail);
      },
    },
  );

  const [totpCode, setTotpCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleVerify = useCallback(() => {
    if (!isPresent(totpCode)) {
      setError('Enter code');
      return;
    }
    if (totpCode.replaceAll(' ', '').length !== 6) {
      setError('6 digits are required');
      return;
    }
    verifyCode(totpCode);
  }, [totpCode, verifyCode]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect of code input
  useEffect(() => {
    setError(null);
  }, [totpCode, setError]);

  // Reflect server-side verify errors into the local error state
  useEffect(() => {
    if (verifyError) setError(verifyError);
  }, [verifyError]);

  // Show loader when posture is being evaluated
  const showLoader = location.posture_check_required && isStarting && !startError;
  if (showLoader) {
    return <LocationCardMfaStartLoader />;
  }

  return (
    <div
      className="location-card-mfa-totp-view"
      onKeyDown={(e) => {
        if (e.key === 'Enter') handleVerify();
      }}
    >
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title="Two-factor authentication">
        <p>Paste the code from your Authenticator Application.</p>
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <CodeInput
        length={6}
        value={totpCode}
        onChange={setTotpCode}
        error={startError ?? error}
      />
      <Controls>
        <IconButton
          variant={IconButtonVariant.BigSelected}
          icon={IconKind.ArrowBig}
          iconRotation="left"
          onClick={() => {
            setView(LocationCardViews.Default);
          }}
        />
        <div className="right">
          <Button
            text="Other methods"
            variant={ButtonVariant.Outlined}
            onClick={() => {
              setView(LocationCardViews.MfaSettings);
            }}
          />
          <Button
            text="Verify"
            variant={ButtonVariant.Primary}
            onClick={handleVerify}
            loading={isVerifying}
            disabled={isStarting}
          />
        </div>
      </Controls>
    </div>
  );
};
