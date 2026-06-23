import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { api } from '../../../../rust-api/api';
import {
  LocationMfaMode,
  MfaMethod,
  type MfaMethodValue,
} from '../../../../rust-api/types';
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
import { MfaSelector } from '../../components/MfaSelector/MfaSelector';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const LocationCardMfaSettings = () => {
  const { mutate: setMfaMethod } = useMutation({
    mutationFn: api.setLocationMfaMethod,
    meta: {
      invalidate: [['locations']],
    },
  });

  const {
    previousView,
    setView,
    location,
    mfaMethod: currentMethod,
    setMfaMethod: setContextMethod,
  } = useLocationCardContext();

  const locationDefaultMfaMethod = location.mfa_method ?? MfaMethod.Totp;

  const [selectedMethod, setSelectedPref] = useState<MfaMethodValue>(currentMethod);

  const isFromDefault = previousView === LocationCardViews.Default;
  const [setAsDefault, setSetAsDefault] = useState(true);

  const MfaFactorsList = useMemo((): MfaMethodValue[] => {
    if (location.location_mfa_mode === LocationMfaMode.Internal) {
      return [MfaMethod.Totp, MfaMethod.Email, MfaMethod.MobileApprove];
    }
    return [MfaMethod.Oidc];
  }, [location.location_mfa_mode]);

  const handleSubmit = () => {
    setContextMethod(selectedMethod);
    if ((isFromDefault || setAsDefault) && selectedMethod !== locationDefaultMfaMethod) {
      setMfaMethod({
        locationId: location.id,
        mfaMethod: selectedMethod,
      });
    }
    switch (selectedMethod) {
      case 'totp':
        setView(LocationCardViews.MfaTotp);
        break;
      case 'email':
        setView(LocationCardViews.MfaEmail);
        break;
      case 'mobileapprove':
        setView(LocationCardViews.MfaMobile);
        break;
      case 'oidc':
        setView(LocationCardViews.MfaOidc);
        break;
      default:
        setView(LocationCardViews.Default);
    }
  };

  return (
    <div className="location-card-mfa-settings">
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title="Change MFA Method">
        <p>
          If you're having issues with your current verification method, you can choose
          another one or set a new default.
        </p>
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="methods">
        {MfaFactorsList.map((factor) => (
          <MfaSelector
            key={factor}
            factor={factor}
            selected={selectedMethod === factor}
            isDefault={locationDefaultMfaMethod === factor}
            onClick={() => setSelectedPref(factor)}
          />
        ))}
      </div>
      {!isFromDefault && (
        <Checkbox
          active={isFromDefault ? true : setAsDefault}
          onClick={() => setSetAsDefault((prev) => !prev)}
          text="Set as default MFA method"
        />
      )}
      <Controls>
        <IconButton
          variant={IconButtonVariant.BigSelected}
          icon={IconKind.ArrowBig}
          iconRotation="left"
          onClick={() => {
            setView(LocationCardViews.Default);
          }}
        />
        <div className="right">
          <Button
            variant={ButtonVariant.Primary}
            size={'primary'}
            text="Save changes"
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
