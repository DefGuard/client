import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useState } from 'react';
import { getAppConfigQueryOptions } from '../../../../rust-api/query';
import { ThemeSpacing } from '../../../../types';
import { findOpenIdProvider, openIdProviderName } from '../../../../utils/mfa';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { ExternalProviderButton } from '../../../ExternalProviderButton/ExternalProviderButton';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { useMfaOidcConnect } from '../../hooks/useMfaOidcConnect';

type Screen = 'idle' | 'polling' | 'error';

export const LocationCardMfaOidcView = () => {
  const { data: appConfig } = useQuery(getAppConfigQueryOptions);
  const {
    setView,
    setPostureError,
    autoConnectOpenid,
    stepLabel,
    instance,
    canPickOtherMethod,
  } = useLocationCardContext();
  const providerName = openIdProviderName(instance);
  const showOtherMethods =
    canPickOtherMethod && appConfig?.auto_start_openid_mfa === false;
  const { start, isStarting, startError, isPolling, pollError } =
    useMfaOidcConnect(autoConnectOpenid);
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

  return (
    <div className="location-card-mfa-oidc">
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title={stepLabel ?? 'Multi-factor authentication'}>
        {screen === 'idle' && (
          <p>
            {`To connect to the VPN, sign in with ${providerName} by clicking button below. A browser window will open and redirect you to ${providerName} for authentication.`}
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
          {showOtherMethods && (
            <Button
              text="Other methods"
              variant={ButtonVariant.Outlined}
              onClick={() => {
                setView(LocationCardViews.MfaSettings);
              }}
            />
          )}
          {screen !== 'error' && (
            <ExternalProviderButton
              text={`Authenticate with ${providerName}`}
              provider={findOpenIdProvider(instance) ?? 'custom'}
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
