import { useCallback, useEffect, useState } from 'react';
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
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { useMfaFido2Connect } from '../../hooks/useMfaFido2Connect';

export const LocationCardMfaFido2View = () => {
  const { setView, location, stepLabel, canPickOtherMethod } = useLocationCardContext();
  const { verifyPin, isVerifying, verifyError } = useMfaFido2Connect(location, {
    onConnected: () => setView(LocationCardViews.Connected),
  });

  const [pin, setPin] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleVerify = useCallback(() => {
    if (!isPresent(pin) || pin.length === 0) {
      setError('Enter PIN');
      return;
    }
    verifyPin(pin);
  }, [pin, verifyPin]);

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
      <LocationViewHeader title={stepLabel ?? 'Two-factor authentication'}>
        <p>Insert your security key and enter its PIN to continue.</p>
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <Input
        type="password"
        label="PIN"
        value={pin}
        onChange={(value) => setPin(isPresent(value) ? String(value) : null)}
        error={error}
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
            text="Verify"
            variant={ButtonVariant.Primary}
            onClick={handleVerify}
            loading={isVerifying}
          />
        </div>
      </Controls>
    </div>
  );
};
