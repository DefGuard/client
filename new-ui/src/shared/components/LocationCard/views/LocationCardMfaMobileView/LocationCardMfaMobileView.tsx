import './style.scss';
import { useEffect, useRef } from 'react';
import { ThemeSpacing } from '../../../../types';
import { isPresent } from '../../../../utils/isPresent';
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

export const LocationCardMfaMobileView = () => {
  const {
    setView,
    showFailure,
    location,
    stepLabel,
    canPickOtherMethod,
    stepPlan,
    mfaToken,
    setMfaToken,
    goToStep,
  } = useLocationCardContext();
  const { start, qrValue } = useMfaMobileConnect(location, {
    stepPlan,
    mfaToken,
    setMfaToken,
    onStepAdvanced: goToStep,
    onConnected: () => setView(LocationCardViews.Connected),
    onFailure: showFailure,
  });
  const startedRef = useRef(false);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    void start();
  }, [start]);

  return (
    <div className="location-card-mfa-mobile">
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title={stepLabel ?? 'Multi-factor authentication'}>
        {!isPresent(qrValue) && <p>Preparing authentication...</p>}
        {isPresent(qrValue) && (
          <p>Open your Defguard mobile app and scan the QR code you see below.</p>
        )}
      </LocationViewHeader>
      {isPresent(qrValue) && (
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
          {canPickOtherMethod && (
            <Button
              variant={ButtonVariant.Outlined}
              text="Other methods"
              onClick={() => {
                setView(LocationCardViews.MfaSettings);
              }}
            />
          )}
        </div>
      </Controls>
    </div>
  );
};
