import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Fragment, useCallback, useEffect, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Checkbox } from '../../../../../../../shared/components/Checkbox/Checkbox';
import { SizedBox } from '../../../../../../../shared/components/SizedBox/SizedBox';
import { api } from '../../../../../../../shared/rust-api/api';
import { getAppConfigQueryOptions } from '../../../../../../../shared/rust-api/query';
import { ThemeSpacing } from '../../../../../../../shared/types';
import { isPresent } from '../../../../../../../shared/utils/isPresent';
import { ConnectModalPostureCheckLoading } from '../../components/ConnectModalPostureCheckLoading/ConnectModalPostureCheckLoading';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';
import { useConnectModalMfaOidc } from '../../hooks/useConnectModalMfaOidc';

type Screen = 'idle' | 'polling' | 'error';

export const ConnectModalMfaOidc = () => {
  const { data: appConfig } = useQuery(getAppConfigQueryOptions);
  const [perviousView, location, initAutoStart] = useConnectModal(
    useShallow((s) => [s.perviousView, s.location, s.autoStartOpenId]),
  );

  const { start, isStarting, startError, isPolling, pollError } = useConnectModalMfaOidc({
    onSessionExpired: () =>
      useConnectModal.getState().setView(perviousView ?? ConnectModalView.MfaSettings),
    onPostureError: (msg) => {
      useConnectModal.setState({ postureError: msg });
      useConnectModal.getState().setView(ConnectModalView.PostureCheckFail);
    },
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

  // biome-ignore lint/correctness/useExhaustiveDependencies: on mount side effect
  useEffect(() => {
    if (initAutoStart) {
      handleStart();
    }
  }, [initAutoStart]);

  if (isStarting && location?.posture_check_required && !startError) {
    return <ConnectModalPostureCheckLoading />;
  }

  return (
    <div id="mfa-oidc-view">
      {screen === 'idle' && (
        <p className="view-description">
          To connect to the VPN, authenticate via your OpenID provider. A browser window
          will open for you to sign in.
        </p>
      )}
      {screen === 'polling' && (
        <p className="view-description">
          Complete the sign-in in your browser. This page will update automatically.
        </p>
      )}
      {screen === 'error' && <p className="view-description">{errorMessage}</p>}
      <div className="actions">
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
            onClick={() => setScreen('idle')}
          />
        )}
        {isPresent(appConfig) && screen === 'idle' && !initAutoStart && (
          <Fragment>
            <SizedBox height={ThemeSpacing.Xl3} />
            <Checkbox
              active={appConfig.auto_start_openid_mfa}
              text={`Don't show this screen next time`}
              onClick={() => {
                void api.setAppConfig(
                  {
                    auto_start_openid_mfa: !appConfig.auto_start_openid_mfa,
                  },
                  true,
                );
              }}
            />
          </Fragment>
        )}
      </div>
    </div>
  );
};
