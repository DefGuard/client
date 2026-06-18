import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { Fragment, useMemo, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Checkbox } from '../../../../../../../shared/components/Checkbox/Checkbox';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { IconKind } from '../../../../../../../shared/components/Icon';
import { IconButton } from '../../../../../../../shared/components/IconButton/IconButton';
import { IconButtonVariant } from '../../../../../../../shared/components/IconButton/types';
import { MfaSelector } from '../../../../../../../shared/components/LocationCard/components/MfaSelector/MfaSelector';
import { SizedBox } from '../../../../../../../shared/components/SizedBox/SizedBox';
import { useAppData } from '../../../../../../../shared/providers/AppDataContext';
import { api } from '../../../../../../../shared/rust-api/api';
import {
  LocationMfaMode,
  MfaMethod,
  type MfaMethodValue,
} from '../../../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../../../shared/types';
import { ConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';

export const ConnectModalMfaSettings = () => {
  const { mutate: setMfaMethod } = useMutation({
    mutationFn: api.setLocationMfaMethod,
    meta: { invalidate: [['locations']] },
  });

  const { locationMfaPreference, setLocationMfaPreference } = useAppData();

  const [perviousView, location] = useConnectModal(
    useShallow((s) => [s.perviousView, s.location]),
  );

  const locationDefaultMfaMethod = location?.mfa_method ?? MfaMethod.Totp;

  const [selectedMethod, setSelectedMethod] = useState<MfaMethodValue>(
    location
      ? (locationMfaPreference[String(location.id)] ?? MfaMethod.Totp)
      : MfaMethod.Totp,
  );
  const [setAsDefault, setSetAsDefault] = useState(true);

  const MfaFactorsList = useMemo((): MfaMethodValue[] => {
    if (location?.location_mfa_mode === LocationMfaMode.Internal) {
      return [MfaMethod.Totp, MfaMethod.Email, MfaMethod.MobileApprove];
    }
    return [MfaMethod.Oidc];
  }, [location?.location_mfa_mode]);

  const handleSubmit = () => {
    if (!location) return;
    setLocationMfaPreference(location.id, selectedMethod);
    if (setAsDefault && selectedMethod !== locationDefaultMfaMethod && location) {
      setMfaMethod({ locationId: location.id, mfaMethod: selectedMethod });
    }
    if (perviousView === null) {
      useConnectModal.setState({ visible: false });
    } else {
      switch (selectedMethod) {
        case 'totp':
          useConnectModal.setState({ view: ConnectModalView.MfaTotp });
          break;
        case 'email':
          useConnectModal.setState({ view: ConnectModalView.MfaEmail });
          break;
        case 'mobileapprove':
          useConnectModal.setState({ view: ConnectModalView.MfaMobile });
          break;
        case 'oidc':
          useConnectModal.setState({ view: ConnectModalView.MfaOidc });
          break;
        default:
          useConnectModal.setState({ visible: false });
          break;
      }
    }
  };

  return (
    <div id="mfa-settings-view">
      {perviousView !== null && (
        <p className="view-description">
          If you're having issues with your current verification method, you can choose
          another one or set a new default.
        </p>
      )}
      {perviousView === null && (
        <p className="view-description">
          {`You can change the MFA method for a one-time login or set a new default method.`}
        </p>
      )}
      <div className="methods">
        {MfaFactorsList.map((factor) => (
          <MfaSelector
            key={factor}
            factor={factor}
            selected={selectedMethod === factor}
            isDefault={locationDefaultMfaMethod === factor}
            onClick={() => setSelectedMethod(factor)}
          />
        ))}
      </div>
      {perviousView !== null && (
        <Fragment>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Checkbox
            active={setAsDefault}
            onClick={() => setSetAsDefault((prev) => !prev)}
            text="Set as default MFA method"
          />
          <SizedBox height={ThemeSpacing.Xl2} />
        </Fragment>
      )}
      {perviousView === null && <SizedBox height={ThemeSpacing.Xl3} />}
      <Controls>
        {perviousView !== null && (
          <IconButton
            variant={IconButtonVariant.BigSelected}
            icon={IconKind.ArrowBig}
            iconRotation="left"
            onClick={() => useConnectModal.getState().setView(perviousView)}
          />
        )}
        <div className="right">
          <Button
            variant={ButtonVariant.Primary}
            size="primary"
            text={perviousView === null ? 'Save changes' : 'Continue'}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
