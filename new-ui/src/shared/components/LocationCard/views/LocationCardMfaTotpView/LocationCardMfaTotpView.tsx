import { useCallback, useEffect, useState } from 'react';
import { ThemeSpacing } from '../../../../types';
import { isPresent } from '../../../../utils/isPresent';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { CodeInput } from '../../../CodeInput/CodeInput';
import { Controls } from '../../../Controls/Controls';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const LocationCardMfaTotpView = () => {
  const { setView } = useLocationCardContext();

  const [totpCode, setTotpCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleVerify = useCallback(() => {
    if (!isPresent(totpCode)) {
      setError('Enter code');
    }
    if (totpCode?.length !== 6) {
      setError('6 digits are required');
    }
  }, [totpCode]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect of code input
  useEffect(() => {
    setError(null);
  }, [totpCode, setError]);

  return (
    <div
      className="location-card-mfa-totp-view"
      onKeyDown={(e) => {
        if (e.key === 'Enter') handleVerify();
      }}
    >
      <LocationViewHeader title="Two-factor authentication">
        <p>Paste the code from your Authenticator Application.</p>
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <CodeInput length={6} value={totpCode} onChange={setTotpCode} error={error} />
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
          <Button text="Verify" variant={ButtonVariant.Primary} onClick={handleVerify} />
        </div>
      </Controls>
    </div>
  );
};
