import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { ExternalProviderButton } from '../../../../../../../shared/components/ExternalProviderButton/ExternalProviderButton';
import { getAppConfigQueryOptions } from '../../../../../../../shared/rust-api/query';
import {
  findOpenIdProvider,
  openIdProviderName,
} from '../../../../../../../shared/utils/mfa';
import { ConnectModalPostureCheckLoading } from '../../components/ConnectModalPostureCheckLoading/ConnectModalPostureCheckLoading';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';
import { useConnectModalMfaOidc } from '../../hooks/useConnectModalMfaOidc';
import { useMfaStep } from '../../hooks/useMfaStep';

type Screen = 'idle' | 'polling' | 'error';

export const ConnectModalMfaOidc = () => {
  const { data: appConfig } = useQuery(getAppConfigQueryOptions);
  const [perviousView, location, initAutoStart] = useConnectModal(
    useShallow((s) => [s.perviousView, s.location, s.autoStartOpenId]),
  );
  const { instance, canPickOtherMethod } = useMfaStep();
  const showOtherMethods =
    canPickOtherMethod && appConfig?.auto_start_openid_mfa === false;
  const providerName = openIdProviderName(instance);

  const { start, isStarting, startError, isPolling, pollError } = useConnectModalMfaOidc({
    autoStart: initAutoStart,
    onSessionExpired: () =>
      useConnectModal.getState().setView(perviousView ?? ConnectModalView.MfaSettings),
    onPostureError: (msg) => {
      useConnectModal.setState({ postureError: msg });
      useConnectModal.getState().setView(ConnectModalView.PostureCheckFail);
    },
    onServiceUnavailable: () =>
      useConnectModal.getState().setView(ConnectModalView.ConnectionError),
  });

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

  if (isStarting && location?.posture_check_required && !startError) {
    return <ConnectModalPostureCheckLoading />;
  }

  return (
    <div id="mfa-oidc-view">
      {screen === 'idle' && (
        <p className="view-description">
          {`To connect to the VPN, sign in with ${providerName} by clicking button below. A browser window will open and redirect you to ${providerName} for authentication.`}
        </p>
      )}
      {screen === 'polling' && (
        <p className="view-description">
          Complete the sign-in in your browser. This page will update automatically.
        </p>
      )}
      {screen === 'error' && <p className="view-description">{errorMessage}</p>}
      <Controls>
        {showOtherMethods && (
          <Button
            variant={ButtonVariant.Secondary}
            text="Other methods"
            onClick={() => {
              useConnectModal.getState().setView(ConnectModalView.MfaSettings);
            }}
          />
        )}
        <div className="right">
          {screen !== 'error' && (
            <ExternalProviderButton
              text={`Continue with ${providerName}`}
              provider={findOpenIdProvider(instance) ?? 'custom'}
              loading={screen === 'polling' || isStarting}
              onClick={handleStart}
            />
          )}
          {screen === 'error' && (
            <Button
              text="Try again"
              variant={ButtonVariant.Primary}
              onClick={() => setScreen('idle')}
            />
          )}
        </div>
      </Controls>
    </div>
  );
};
