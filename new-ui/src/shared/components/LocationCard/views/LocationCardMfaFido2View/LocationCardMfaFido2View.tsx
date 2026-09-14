import { Fragment, useCallback, useEffect, useState } from 'react';
import { fido2CollectsPinInApp } from '../../../../rust-api/fido2';
import { ThemeSpacing } from '../../../../types';
import { isPresent } from '../../../../utils/isPresent';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { Input } from '../../../Input/Input';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { Fido2TouchPrompt } from '../../components/Fido2TouchPrompt/Fido2TouchPrompt';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { useMfaFido2Connect } from '../../hooks/useMfaFido2Connect';

export const LocationCardMfaFido2View = () => {
  const {
    setView,
    location,
    stepLabel,
    canPickOtherMethod,
    stepPlan,
    mfaToken,
    setPostureError,
  } = useLocationCardContext();
  const { verify, isVerifying, isAwaitingTouch, verifyError } = useMfaFido2Connect(
    location,
    {
      stepPlan,
      mfaToken,
      onConnected: () => setView(LocationCardViews.Connected),
      onPostureError: (message) => {
        setPostureError(message);
        setView(LocationCardViews.PostureCheckFail);
      },
      onServiceUnavailable: () => setView(LocationCardViews.ConnectionError),
    },
  );

  const [pin, setPin] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const collectsPin = fido2CollectsPinInApp();

  const handleVerify = useCallback(() => {
    if (!collectsPin) {
      verify(null);
      return;
    }
    if (!isPresent(pin) || pin.length === 0) {
      setError('Enter PIN');
      return;
    }
    verify(pin);
  }, [collectsPin, pin, verify]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect of pin input
  useEffect(() => {
    setError(null);
  }, [pin, setError]);

  // Reflect backend errors into the local error state
  useEffect(() => {
    if (verifyError) setError(verifyError);
  }, [verifyError]);

  return (
    <div
      className="location-card-mfa-fido2-view"
      onKeyDown={(e) => {
        if (e.key === 'Enter') handleVerify();
      }}
    >
      <Divider spacing={ThemeSpacing.Md} />
      {isAwaitingTouch ? (
        <Fido2TouchPrompt />
      ) : (
        <Fragment>
          <LocationViewHeader title={stepLabel ?? 'Multi-factor authentication'}>
            <p>
              {collectsPin
                ? 'Insert your security key and enter its PIN to continue.'
                : 'Insert your security key and continue in the prompt your system shows.'}
            </p>
          </LocationViewHeader>
          <SizedBox height={ThemeSpacing.Xl} />
          {collectsPin ? (
            <Input
              type="password"
              label="PIN"
              value={pin}
              onChange={(value) => setPin(isPresent(value) ? String(value) : null)}
              error={error}
            />
          ) : (
            isPresent(error) && <p className="error">{error}</p>
          )}
        </Fragment>
      )}
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
          {canPickOtherMethod && (
            <Button
              text="Other methods"
              variant={ButtonVariant.Outlined}
              onClick={() => {
                setView(LocationCardViews.MfaSettings);
              }}
            />
          )}
          <Button
            text={collectsPin ? 'Verify' : 'Use security key'}
            variant={ButtonVariant.Primary}
            onClick={handleVerify}
            loading={isVerifying}
          />
        </div>
      </Controls>
    </div>
  );
};
