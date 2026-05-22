import './style.scss';
import { useEffect, useState } from 'react';
import { ThemeSpacing } from '../../../../types';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { useMfaOidcConnect } from '../../hooks/useMfaOidcConnect';

type Screen = 'idle' | 'polling' | 'error';

export const LocationCardMfaOidcView = () => {
  const { setView, setPostureError } = useLocationCardContext();
  const { start, isStarting, startError, isPolling, pollError } = useMfaOidcConnect();
  const [screen, setScreen] = useState<Screen>('idle');

  useEffect(() => {
    if (startError ?? pollError) {
      setScreen((prev) => (prev !== 'idle' ? 'error' : prev));
    } else if (isPolling) {
      setScreen('polling');
    }
  }, [startError, pollError, isPolling]);

  const handleStart = async () => {
    await start();
    setScreen('polling');
  };

  const errorMessage = startError ?? pollError;

  const backToLocation = () => {
    setPostureError(null);
    setView(LocationCardViews.Default);
  };

  return (
    <div className="location-card-mfa-oidc">
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title="Two-factor authentication">
        {screen === 'idle' && (
          <p>
            To connect to the VPN, authenticate via your OpenID provider. A browser window
            will open for you to sign in.
          </p>
        )}
        {screen === 'polling' && (
          <p>
            {`Complete the sign-in in your browser. This page will update automatically.`}
          </p>
        )}
        {screen === 'error' && <p className="error">{errorMessage}</p>}
      </LocationViewHeader>
      <Controls>
        <IconButton
          variant={IconButtonVariant.BigSelected}
          icon={IconKind.ArrowBig}
          iconRotation="left"
          onClick={() => setView(LocationCardViews.Default)}
        />
        <div className="right">
          {screen !== 'error' && (
            <Button
              text="Auth with OpenID"
              variant={ButtonVariant.Primary}
              loading={screen === 'polling' || isStarting}
              onClick={handleStart}
            />
          )}
          {screen === 'error' && (
            <Button
              text="Try again"
              variant={ButtonVariant.Primary}
              onClick={backToLocation}
            />
          )}
        </div>
      </Controls>
    </div>
  );
};
