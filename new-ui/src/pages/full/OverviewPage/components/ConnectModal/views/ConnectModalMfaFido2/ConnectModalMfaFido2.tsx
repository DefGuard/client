import { useCallback, useEffect, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { InfoBanner } from '../../../../../../../shared/components/InfoBanner/InfoBanner';
import { Input } from '../../../../../../../shared/components/Input/Input';
import { useMfaFido2Connect } from '../../../../../../../shared/components/LocationCard/hooks/useMfaFido2Connect';
import { SizedBox } from '../../../../../../../shared/components/SizedBox/SizedBox';
import type { LocationInfo } from '../../../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../../../shared/types';
import { isPresent } from '../../../../../../../shared/utils/isPresent';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';

export const ConnectModalMfaFido2 = () => {
  const [location] = useConnectModal(useShallow((s) => [s.location]));

  const { verifyPin, isVerifying, verifyError, message } = useMfaFido2Connect(
    location as LocationInfo,
  );

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
      <p className="view-description">
        Insert your security key and enter its PIN to continue.
      </p>
      <Input
        type="password"
        label="PIN"
        value={pin}
        onChange={(value) => setPin(value === null ? null : String(value))}
        error={error}
        autocomplete="off"
      />
      {isPresent(message) && (
        <>
          <InfoBanner message={message} />
          <SizedBox height={ThemeSpacing.Xl} />
        </>
      )}
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
            onClick={handleVerify}
            loading={isVerifying}
          />
        </div>
      </Controls>
    </div>
  );
};
