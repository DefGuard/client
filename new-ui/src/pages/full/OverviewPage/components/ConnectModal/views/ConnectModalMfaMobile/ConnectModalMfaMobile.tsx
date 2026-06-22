import './style.scss';
import { useEffect, useRef, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { useMfaMobileConnect } from '../../../../../../../shared/components/LocationCard/hooks/useMfaMobileConnect';
import { QrCard } from '../../../../../../../shared/components/QrCard/QrCard';
import type { LocationInfo } from '../../../../../../../shared/rust-api/types';
import { ConnectModalPostureCheckLoading } from '../../components/ConnectModalPostureCheckLoading/ConnectModalPostureCheckLoading';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';

type Screen = 'loading' | 'qr' | 'error';

export const ConnectModalMfaMobile = () => {
  const [_perviousView, location] = useConnectModal(
    useShallow((s) => [s.perviousView, s.location]),
  );

  const { start, isStarting, startError, qrValue, connectionError } = useMfaMobileConnect(
    location as LocationInfo,
    {
      onPostureError: (msg) => {
        useConnectModal.setState({ postureError: msg });
        useConnectModal.getState().setView(ConnectModalView.PostureCheckFail);
      },
    },
  );

  const [screen, setScreen] = useState<Screen>('loading');
  const startedRef = useRef(false);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    void start();
  }, [start]);

  useEffect(() => {
    if (isStarting) {
      setScreen('loading');
    } else if (startError ?? connectionError) {
      setScreen('error');
    } else if (qrValue) {
      setScreen('qr');
    }
  }, [isStarting, startError, connectionError, qrValue]);

  const errorMessage = startError ?? connectionError;

  if (isStarting && location?.posture_check_required && !startError) {
    return <ConnectModalPostureCheckLoading />;
  }

  return (
    <div id="mfa-mobile-view">
      <p className="view-description">
        {screen === 'loading' && 'Preparing authentication...'}
        {screen === 'qr' && 'Open your Defguard mobile app and scan the QR code below.'}
        {screen === 'error' && <span className="error">{errorMessage}</span>}
      </p>
      {screen === 'qr' && qrValue && (
        <div className="qr-track">
          <QrCard value={qrValue} size={184} />
        </div>
      )}
      <Controls>
        {screen === 'qr' && (
          <Button
            containerProps={{ className: 'full' }}
            variant={ButtonVariant.Secondary}
            text="Use different MFA"
            onClick={() => {
              useConnectModal.getState().setView(ConnectModalView.MfaSettings);
            }}
          />
        )}
        {screen === 'error' && (
          <Button
            containerProps={{ className: 'full' }}
            text="Try again"
            variant={ButtonVariant.Primary}
            onClick={start}
          />
        )}
      </Controls>
    </div>
  );
};
