import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { api } from '../../../../rust-api/api';
import type { MfaMethodValue } from '../../../../rust-api/types';
import { ThemeSpacing } from '../../../../types';
import { isPresent } from '../../../../utils/isPresent';
import {
  mfaStepCount,
  pickableMfaMethods,
  resolveMfaStepPlan,
  usableMfaMethods,
} from '../../../../utils/mfa';
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
    setMfaMethod: setContextMethod,
    stepPlan,
    stepIndex,
    setStepPlanOnce,
  } = useLocationCardContext();

  const mfaSteps = location.mfa_steps;
  const isMultiStep = mfaStepCount(location) > 1;
  const defaultPlan = resolveMfaStepPlan(location);

  const isEditingDefaults = previousView === LocationCardViews.Default;

  const [selectedStepMethods, setSelectedStepMethods] = useState<MfaMethodValue[]>(
    isEditingDefaults ? defaultPlan : stepPlan,
  );
  const [saveAsDefault, setSaveAsDefault] = useState(false);

  const editableSteps = useMemo(() => {
    if (isEditingDefaults) {
      return mfaSteps.map((step, index) => ({
        stepIndex: index,
        methods: pickableMfaMethods(step),
      }));
    }
    const currentStep = mfaSteps[stepIndex];
    if (!isPresent(currentStep)) return [];
    return [{ stepIndex, methods: usableMfaMethods(currentStep) }];
  }, [isEditingDefaults, mfaSteps, stepIndex]);

  const selectMethodForStep = (targetStepIndex: number, method: MfaMethodValue) => {
    setSelectedStepMethods((currentPlan) =>
      currentPlan.map((selected, index) =>
        index === targetStepIndex ? method : selected,
      ),
    );
  };

  const handleSubmit = () => {
    if (isEditingDefaults) {
      setMfaStepPlan({ locationId: location.id, mfaStepPlan: selectedStepMethods });
      setView(LocationCardViews.Default);
      return;
    }

    if (!isMultiStep && saveAsDefault) {
      setMfaStepPlan({ locationId: location.id, mfaStepPlan: selectedStepMethods });
    }

    const methodForCurrentStep = selectedStepMethods[stepIndex];
    setStepPlanOnce(selectedStepMethods);
    setContextMethod(methodForCurrentStep);
    setView(mfaMethodToLocationCardView(methodForCurrentStep));
  };

  return (
    <div className="location-card-mfa-settings">
      <Divider spacing={ThemeSpacing.Md} />
      <LocationViewHeader title="Change MFA Method">
        {isEditingDefaults && (
          <p>{`Choose the default verification method for each step of this location.`}</p>
        )}
        {!isEditingDefaults && (
          <p>
            {isMultiStep
              ? `If you're having issues with your current verification method, you can choose another one for this login.`
              : `If you're having issues with your current verification method, you can choose another one or set a new default.`}
          </p>
        )}
      </LocationViewHeader>
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="steps">
        {editableSteps.map(({ methods, stepIndex: index }) => (
          <div className="step" key={index}>
            {isMultiStep && isEditingDefaults && (
              <p className="step-label">Step {index + 1}</p>
            )}
            <div className="methods">
              {methods.map((entry) => (
                <MfaSelector
                  key={entry.method}
                  factor={entry.method}
                  selected={selectedStepMethods[index] === entry.method}
                  isDefault={defaultPlan[index] === entry.method}
                  configured={entry.configured}
                  onClick={() => selectMethodForStep(index, entry.method)}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
      {!isEditingDefaults && !isMultiStep && (
        <Checkbox
          active={saveAsDefault}
          onClick={() => setSaveAsDefault((current) => !current)}
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
            text={isEditingDefaults ? 'Save changes' : 'Continue'}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
