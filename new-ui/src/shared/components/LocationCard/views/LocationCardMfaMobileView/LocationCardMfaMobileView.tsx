import './style.scss';
import { useEffect, useRef, useState } from 'react';
import { ThemeSpacing } from '../../../../types';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { QrCard } from '../../../QrCard/QrCard';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';
import { useMfaMobileConnect } from '../../hooks/useMfaMobileConnect';

type Screen = 'loading' | 'qr' | 'error';

export const LocationCardMfaMobileView = () => {
  const { setView, setPostureError, location } = useLocationCardContext();
  const { start, startError, qrValue, connectionError } = useMfaMobileConnect(location, {
    onConnected: () => setView(LocationCardViews.Connected),
    onPostureError: (message) => setPostureError(message ?? null),
  });
  const [screen, setScreen] = useState<Screen>('loading');
  const startedRef = useRef(false);

  // Auto-start on mount
  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    void start();
  }, [start]);

  useEffect(() => {
    if (startError ?? connectionError) {
      setScreen('error');
    } else if (qrValue) {
      setScreen('qr');
    }
  }, [startError, connectionError, qrValue]);

  const backToLocation = () => {
    setPostureError(null);
    setView(LocationCardViews.Default);
  };

  const errorMessage = startError ?? connectionError;

  return (
    <div className="location-card-mfa-mobile">
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title="Two-factor authentication">
        {screen === 'loading' && <p>Preparing authentication...</p>}
        {screen === 'qr' && (
          <p>Open your Defguard mobile app and scan the QR code you see bellow.</p>
        )}
        {screen === 'error' && <p className="error">{errorMessage}</p>}
      </LocationViewHeader>
      {screen === 'qr' && qrValue && (
        <div className="qr-wrapper">
          <QrCard value={qrValue} size={184} />
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
