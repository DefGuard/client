import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { api } from '../../../../rust-api/api';
import { LocationMfaMode, MfaMethod, type MfaMethodValue } from '../../../../rust-api/types';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Checkbox } from '../../../Checkbox/Checkbox';
import { Controls } from '../../../Controls/Controls';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
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

  const { previousView, setView, location } = useLocationCardContext();

  const mfaMethod = location.mfa_method ?? MfaMethod.Totp;

  const [selectedPref, setSelectedPref] = useState<MfaMethodValue>(
    mfaMethod ?? MfaMethod.Totp,
  );

  const MfaFactorsList = useMemo((): MfaMethodValue[] => {
    if(location.location_mfa_mode === LocationMfaMode.Internal) {
      return [
        MfaMethod.Totp,
        MfaMethod.Email,
        MfaMethod.MobileApprove,
      ];
    }
    return [MfaMethod.Oidc]
  }, [location.location_mfa_mode]);

  const handleSubmit = () => {
    if (selectedPref !== mfaMethod) {
      setMfaMethod({
        locationId: location.id,
        mfaMethod: selectedPref,
      });
      setView(previousView ?? LocationCardViews.Default);
    }
  };

  return (
    <div className="location-card-mfa-settings">
      <div className="header">
        <p>Change MFA Method</p>
        <p>
          If you're having issues with your current verification method, you can choose
          another one or set a new default.
        </p>
      </div>
      <div className="methods">
        {MfaFactorsList.map((factor) => (
          <MfaSelector
            key={factor}
            factor={factor}
            selected={selectedPref === factor}
            isDefault={mfaMethod === factor}
            onClick={() => setSelectedPref(factor)}
          />
        ))}
      </div>
      <Checkbox
        active={
          previousView === LocationCardViews.Default ? true : mfaMethod === selectedPref
        }
        disabled={
          previousView === LocationCardViews.Default ? true : mfaMethod === selectedPref
        }
        text="Set as default MFA method"
      />
      <Controls>
        <IconButton
          variant={IconButtonVariant.BigSelected}
          icon={IconKind.ArrowBig}
          iconRotation="left"
          onClick={() => {
            setView(previousView ?? LocationCardViews.Default);
          }}
        />
        <div className="right">
          <Button
            disabled={selectedPref === mfaMethod}
            variant={ButtonVariant.Primary}
            size={'primary'}
            text="Submit"
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
