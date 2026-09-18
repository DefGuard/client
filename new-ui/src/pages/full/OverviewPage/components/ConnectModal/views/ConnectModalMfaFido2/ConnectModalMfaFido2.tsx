import { Fragment, useCallback, useEffect, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { Input } from '../../../../../../../shared/components/Input/Input';
import { Fido2TouchPrompt } from '../../../../../../../shared/components/LocationCard/components/Fido2TouchPrompt/Fido2TouchPrompt';
import { useMfaFido2Connect } from '../../../../../../../shared/components/LocationCard/hooks/useMfaFido2Connect';
import { fido2CollectsPinInApp } from '../../../../../../../shared/rust-api/fido2';
import type { LocationInfo } from '../../../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../../../shared/utils/isPresent';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';
import { useMfaStep } from '../../hooks/useMfaStep';

export const ConnectModalMfaFido2 = () => {
  const [location] = useConnectModal(useShallow((s) => [s.location]));
  const { canPickOtherMethod, stepPlan, mfaToken, setMfaToken, goToStep } = useMfaStep();

  const { verify, isVerifying, isAwaitingTouch, verifyError } = useMfaFido2Connect(
    location as LocationInfo,
    {
      stepPlan,
      mfaToken,
      setMfaToken,
      onStepAdvanced: goToStep,
      onPostureError: (message) => {
        useConnectModal.setState({ postureError: message });
        useConnectModal.getState().setView(ConnectModalView.PostureCheckFail);
      },
      onServiceUnavailable: () =>
        useConnectModal.getState().setView(ConnectModalView.ConnectionError),
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

  useEffect(() => {
    if (verifyError) setError(verifyError);
  }, [verifyError]);

  return (
    <div
      id="mfa-fido2-view"
      onKeyDown={(e) => {
        if (e.key === 'Enter') handleVerify();
      }}
    >
      {isAwaitingTouch ? (
        <Fido2TouchPrompt />
      ) : (
        <Fragment>
          <p className="view-description">
            {collectsPin
              ? 'Insert your security key and enter its PIN to continue.'
              : 'Insert your security key and continue in the prompt your system shows.'}
          </p>
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
        {canPickOtherMethod && (
          <Button
            variant={ButtonVariant.Secondary}
            text="Other methods"
            onClick={() => {
              useConnectModal.getState().setView(ConnectModalView.MfaSettings);
            }}
          />
        )}
        <div className="right">
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
