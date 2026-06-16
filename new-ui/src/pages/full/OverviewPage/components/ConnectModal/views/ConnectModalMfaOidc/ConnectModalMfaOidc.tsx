import { useEffect, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { ConnectModalPostureCheckLoading } from '../../components/ConnectModalPostureCheckLoading/ConnectModalPostureCheckLoading';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';
import { useConnectModalMfaOidc } from '../../hooks/useConnectModalMfaOidc';

type Screen = 'idle' | 'polling' | 'error';

export const ConnectModalMfaOidc = () => {
  const [perviousView, location] = useConnectModal(
    useShallow((s) => [s.perviousView, s.location]),
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

  const handleStart = async () => {
    await start();
    setScreen('polling');
  };

  const errorMessage = startError ?? pollError;

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
      <Controls>
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
              onClick={() => setScreen('idle')}
            />
          )}
        </div>
      </Controls>
    </div>
  );
};
