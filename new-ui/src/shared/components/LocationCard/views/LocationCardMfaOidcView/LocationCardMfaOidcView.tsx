import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useState } from 'react';
import { api } from '../../../../rust-api/api';
import { getAppConfigQueryOptions } from '../../../../rust-api/query';
import { ThemeSpacing } from '../../../../types';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Checkbox } from '../../../Checkbox/Checkbox';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { useMfaOidcConnect } from '../../hooks/useMfaOidcConnect';

type Screen = 'idle' | 'polling' | 'error';

export const LocationCardMfaOidcView = () => {
  const { data: appConfig } = useQuery(getAppConfigQueryOptions);
  const { setView, setPostureError, autoConnectOpenid } = useLocationCardContext();
  const { start, isStarting, startError, isPolling, pollError } = useMfaOidcConnect();
  const [screen, setScreen] = useState<Screen>('idle');

  useEffect(() => {
    if (startError ?? pollError) {
      setScreen((prev) => (prev !== 'idle' ? 'error' : prev));
    } else if (isPolling) {
      setScreen('polling');
    }
  }, [startError, pollError, isPolling]);

  const handleStart = useCallback(async () => {
    await start();
    setScreen('polling');
  }, [start]);

  const errorMessage = startError ?? pollError;

  const backToLocation = () => {
    setPostureError(null);
    setView(LocationCardViews.Default);
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: on mount effect
  useEffect(() => {
    if (autoConnectOpenid) {
      handleStart();
    }
  }, []);

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
      {screen === 'idle' && !autoConnectOpenid && (
        <div className="actions">
          <SizedBox height={ThemeSpacing.Lg} />
          <Checkbox
            active={appConfig?.auto_start_openid_mfa}
            text={`Don't show this screen next time`}
            onClick={() => {
              void api.setAppConfig(
                {
                  auto_start_openid_mfa: !appConfig?.auto_start_openid_mfa,
                },
                true,
              );
            }}
          />
        </div>
      )}
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
