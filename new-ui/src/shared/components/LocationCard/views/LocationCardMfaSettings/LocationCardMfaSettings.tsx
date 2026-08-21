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
import { isMfaMethodUsable, mfaStepCount } from '../../../../utils/mfa';
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
import { LocationCardViews, mfaMethodToLocationCardView } from '../../context/types';

export const LocationCardMfaSettings = () => {
  const { mutate: setMfaMethod } = useMutation({
    mutationFn: api.setLocationMfaMethod,
    meta: {
      invalidate: [['locations']],
    },
  });

  const { mutate: setMfaStepPlan } = useMutation({
    mutationFn: api.setLocationMfaStepPlan,
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
    stepPlan,
    stepIndex,
    setStepPlanOnce,
  } = useLocationCardContext();

  const mfaSteps = location.mfa_steps;
  const isMultiStep = mfaStepCount(location) > 1;

  const locationDefaultMfaMethod = location.mfa_method ?? MfaMethod.Totp;

  const [selectedMethod, setSelectedPref] = useState<MfaMethodValue>(currentMethod);
  const [selectedStepMethods, setSelectedStepMethods] =
    useState<MfaMethodValue[]>(stepPlan);

  const isFromDefault = previousView === LocationCardViews.Default;
  const [setAsDefault, setSetAsDefault] = useState(true);

  const editedSteps = useMemo(() => {
    const steps = isFromDefault
      ? mfaSteps.map((step, index) => ({ step, index }))
      : [{ step: mfaSteps[stepIndex], index: stepIndex }];
    return steps.map(({ step, index }) => ({
      index,
      // Unconfigured factors stay visible while editing the plan, so they can later be
      // set up in place. Biometrics show up only when a step has nothing else, which shouldn't happen
      // because core won't send such a location, it is here so the step still renders.
      methods: isFromDefault
        ? step.methods.filter(
            (entry) => entry.method !== MfaMethod.Biometric || step.methods.length === 1,
          )
        : step.methods.filter(isMfaMethodUsable),
    }));
  }, [isFromDefault, mfaSteps, stepIndex]);

  const MfaFactorsList = useMemo((): MfaMethodValue[] => {
    if (location.location_mfa_mode === LocationMfaMode.Internal) {
      return [MfaMethod.Totp, MfaMethod.Email, MfaMethod.MobileApprove];
    }
    return [MfaMethod.Oidc];
  }, [location.location_mfa_mode]);

  const selectStepMethod = (stepIndex: number, method: MfaMethodValue) => {
    setSelectedStepMethods((current) =>
      current.map((value, index) => (index === stepIndex ? method : value)),
    );
  };

  const handleSubmit = () => {
    if (isMultiStep) {
      if (isFromDefault) {
        setMfaStepPlan({ locationId: location.id, mfaStepPlan: selectedStepMethods });
        setView(LocationCardViews.Default);
        return;
      }
      setStepPlanOnce(selectedStepMethods);
      setView(mfaMethodToLocationCardView(selectedStepMethods[stepIndex]));
      return;
    }

    setContextMethod(selectedMethod);
    if ((isFromDefault || setAsDefault) && selectedMethod !== locationDefaultMfaMethod) {
      setMfaMethod({
        locationId: location.id,
        mfaMethod: selectedMethod,
      });
    }
    if (isFromDefault) {
      setView(LocationCardViews.Default);
      return;
    }
    setView(mfaMethodToLocationCardView(selectedMethod));
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
      {isMultiStep ? (
        <div className="steps">
          {editedSteps.map(({ methods, index }) => (
            <div className="step" key={index}>
              {isFromDefault && <p className="step-label">Step {index + 1}</p>}
              <div className="methods">
                {methods.map((entry) => (
                  <MfaSelector
                    key={entry.method}
                    factor={entry.method}
                    selected={selectedStepMethods[index] === entry.method}
                    isDefault={stepPlan[index] === entry.method}
                    configured={entry.configured}
                    onClick={() => selectStepMethod(index, entry.method)}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      ) : (
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
      )}
      {!isMultiStep && !isFromDefault && (
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
            text={isMultiStep && !isFromDefault ? 'Confirm' : 'Save changes'}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
