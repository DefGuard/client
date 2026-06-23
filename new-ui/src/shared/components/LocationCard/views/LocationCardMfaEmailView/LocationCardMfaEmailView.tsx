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

export const LocationCardMfaEmailView = () => {
  const { setView, location, setPostureError } = useLocationCardContext();
  const { verifyCode, isVerifying, verifyError, isStarting, startError } = useMfaConnect(
    location,
    MfaStartMethod.Email,
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
      className="location-card-mfa-email-view"
      onKeyDown={(e) => {
        if (e.key === 'Enter') handleVerify();
      }}
    >
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title="Email verification">
        <p>Enter the 6-digit code sent to your email address.</p>
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <CodeInput
        length={6}
        value={emailCode}
        onChange={setEmailCode}
        error={startError ?? error}
        onSuccessPaste={(value) => {
          setEmailCode(value);
          handleVerify();
        }}
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
            variant={ButtonVariant.Outlined}
            text="Other methods"
            onClick={() => {
              setView(LocationCardViews.MfaSettings);
            }}
          />
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
