import './style.scss';
import { Enter } from '@fluentui/keyboard-keys';
import { useState } from 'react';
import { IconKind } from '../../../../../shared/components/Icon';
import { Input } from '../../../../../shared/components/Input/Input';
import { fido2CollectsPinInApp } from '../../../../../shared/rust-api/fido2';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { MfaCodeVerification } from '../../components/MfaCodeVerification/MfaCodeVerification';
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

  let description = collectsPin
    ? 'Insert your key and enter its PIN to continue.'
    : 'Insert your security key and continue in the prompt your system shows.';
  if (isAwaitingTouch) description = 'The key is blinking. Tap it to confirm.';

  return (
    <MfaCodeVerification
      id="configure-verify-fido2-step"
      icon={IconKind.SoftwareKey}
      title="Verify with your security key"
      description={[description]}
      onSubmit={handleVerify}
      submitText="Verify"
      leaveText={leaveText}
      onLeave={() => {
        void handleLeave();
      }}
      isLeaving={isLeaving}
      submitLoading={isVerifying}
    >
      {!isAwaitingTouch && collectsPin && (
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
      {!isAwaitingTouch && !collectsPin && isPresent(error) && (
        <p className="error">{error}</p>
      )}
    </MfaCodeVerification>
  );
};
