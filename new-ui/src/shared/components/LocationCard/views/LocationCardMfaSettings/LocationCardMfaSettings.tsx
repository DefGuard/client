import { useState } from 'react';
import { MfaMethod, type MfaMethodValue } from '../../../../rust-api/types';
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

const MFA_FACTORS: MfaMethodValue[] = [
  MfaMethod.Totp,
  MfaMethod.Email,
  MfaMethod.MobileApprove,
  MfaMethod.Oidc,
];

export const LocationCardMfaSettings = () => {
  const { previousView, mfaMethod, setView, setMfaMethod } = useLocationCardContext();
  const [selectedPref, setSelectedPref] = useState<MfaMethodValue>(
    mfaMethod ?? MfaMethod.Totp,
  );
  const handleSubmit = () => {
    if (selectedPref !== mfaMethod) {
      setMfaMethod(selectedPref);
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
        {MFA_FACTORS.map((factor) => (
          <MfaSelector
            key={factor}
            factor={factor}
            selected={selectedPref === factor}
            isDefault={mfaMethod === factor}
            onClick={() => setSelectedPref(factor)}
          />
        ))}
      </div>
      {/* TODO: Add the pref MFA method to the location model */}
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
          onClick={handleSubmit}
        />
        <div className="right">
          <Button
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
