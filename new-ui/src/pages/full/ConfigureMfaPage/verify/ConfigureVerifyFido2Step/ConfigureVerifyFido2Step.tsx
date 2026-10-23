import './style.scss';
import { Enter } from '@fluentui/keyboard-keys';
import { useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Icon, IconKind } from '../../../../../shared/components/Icon';
import { Input } from '../../../../../shared/components/Input/Input';
import { Fido2TouchPrompt } from '../../../../../shared/components/LocationCard/components/Fido2TouchPrompt/Fido2TouchPrompt';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { fido2CollectsPinInApp } from '../../../../../shared/rust-api/fido2';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { useLeaveVerification } from '../../hooks/useLeaveVerification';
import { useConfigureVerifyFido2 } from './useConfigureVerifyFido2';

interface Props {
  onCancel: () => void;
  onSessionExpired: () => void;
}

export const ConfigureVerifyFido2Step = ({ onCancel, onSessionExpired }: Props) => {
  const { leaveText, leave, isLeaving } = useLeaveVerification(onCancel);
  const collectsPin = fido2CollectsPinInApp();
  const [pin, setPin] = useState<string | null>(null);
  const { verify, abort, isVerifying, isAwaitingTouch, error, setError } =
    useConfigureVerifyFido2({ onSessionExpired, autoStart: !collectsPin });

  const handleVerify = () => {
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
  };

  const handleLeave = async () => {
    // Core holds one pending attempt per session, so abort it before another method starts
    await abort();
    leave();
  };

  return (
    <FullPage id="configure-verify-fido2-step" hideScrollContainer withControls>
      <div className="content">
        {isAwaitingTouch && <Fido2TouchPrompt />}
        {!isAwaitingTouch && (
          <>
            <Icon icon={IconKind.SoftwareKey} size={28} />
            <p className="title">Verify with your security key</p>
            <p className="description">
              {collectsPin
                ? 'Insert your key and enter its PIN to continue.'
                : 'Insert your security key and continue in the prompt your system shows.'}
            </p>
            {collectsPin && (
              <div
                className="pin-track"
                onKeyDown={(e) => {
                  if (e.key === Enter) handleVerify();
                }}
              >
                <Input
                  type="password"
                  autoFocus
                  value={pin}
                  onChange={(value) => {
                    setPin(isPresent(value) ? String(value) : null);
                    setError(null);
                  }}
                  error={error}
                />
              </div>
            )}
            {!collectsPin && isPresent(error) && <p className="error">{error}</p>}
          </>
        )}
      </div>
      <Controls>
        <Button
          text={leaveText}
          variant={ButtonVariant.Secondary}
          loading={isLeaving}
          onClick={() => {
            void handleLeave();
          }}
        />
        <div className="right">
          <Button
            text="Verify"
            variant={ButtonVariant.Primary}
            loading={isVerifying}
            onClick={handleVerify}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
