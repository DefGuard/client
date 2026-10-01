import { Enter } from '@fluentui/keyboard-keys';
import { Fragment, useCallback, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { Input } from '../../../../../shared/components/Input/Input';
import { Fido2TouchPrompt } from '../../../../../shared/components/LocationCard/components/Fido2TouchPrompt/Fido2TouchPrompt';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { fido2CollectsPinInApp } from '../../../../../shared/rust-api/fido2';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';
import { useConfigureVerifyFido2 } from './useConfigureVerifyFido2';
import '../style.scss';

interface Props {
  onSessionExpired: () => void;
}

export const ConfigureVerifyFido2Step = ({ onSessionExpired }: Props) => {
  const collectsPin = fido2CollectsPinInApp();
  const [pin, setPin] = useState<string | null>(null);
  const { verify, abort, isVerifying, isAwaitingTouch, error, setError } =
    useConfigureVerifyFido2({ onSessionExpired, autoStart: !collectsPin });

  const handleVerify = useCallback(() => {
    if (isVerifying) return;
    if (!collectsPin) {
      void verify(null);
      return;
    }
    if (!isPresent(pin) || pin.length === 0) {
      setError('Enter PIN');
      return;
    }
    void verify(pin);
  }, [collectsPin, isVerifying, pin, setError, verify]);

  const handleBack = useCallback(async () => {
    // Core holds one pending attempt per session, so abort it before another method starts
    await abort();
    useConfigureMfaStore.getState().backFromVerification();
  }, [abort]);

  return (
    <FullPage
      id="configure-verify-fido2-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Verify with your security key" />
      {isAwaitingTouch ? (
        <Fido2TouchPrompt />
      ) : (
        <Fragment>
          <p className="description">
            <span>
              {collectsPin
                ? 'Insert your security key and enter its PIN to continue.'
                : 'Insert your security key and continue in the prompt your system shows.'}
            </span>
          </p>
          {collectsPin ? (
            <div
              className="pin-track"
              onKeyDown={(e) => {
                if (e.key === Enter) handleVerify();
              }}
            >
              <Input
                type="password"
                label="PIN"
                value={pin}
                onChange={(value) => {
                  setPin(isPresent(value) ? String(value) : null);
                  setError(null);
                }}
                error={error}
              />
            </div>
          ) : (
            isPresent(error) && <p className="error">{error}</p>
          )}
        </Fragment>
      )}
      <Controls>
        <Button
          text="Back"
          variant={ButtonVariant.Secondary}
          onClick={() => {
            void handleBack();
          }}
        />
        <div className="right">
          <Button
            text={collectsPin ? 'Verify' : 'Use security key'}
            variant={ButtonVariant.Primary}
            loading={isVerifying}
            onClick={handleVerify}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
