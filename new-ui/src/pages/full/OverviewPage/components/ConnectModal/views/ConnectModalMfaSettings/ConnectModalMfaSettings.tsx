import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { Fragment, useMemo, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Checkbox } from '../../../../../../../shared/components/Checkbox/Checkbox';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { MfaSelector } from '../../../../../../../shared/components/LocationCard/components/MfaSelector/MfaSelector';
import { SizedBox } from '../../../../../../../shared/components/SizedBox/SizedBox';
import { api } from '../../../../../../../shared/rust-api/api';
import type { MfaMethodValue } from '../../../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../../../shared/types';
import { isPresent } from '../../../../../../../shared/utils/isPresent';
import {
  mfaStepCount,
  mfaStepsOf,
  pickableMfaMethods,
  resolveMfaStepPlan,
  usableMfaMethods,
} from '../../../../../../../shared/utils/mfa';
import { mfaMethodToConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';

export const ConnectModalMfaSettings = () => {
  const { mutate: setMfaStepPlan } = useMutation({
    mutationFn: api.setLocationMfaStepPlan,
    meta: { invalidate: [['locations']] },
  });

  const [perviousView, location, stepPlan, stepIndex] = useConnectModal(
    useShallow((s) => [s.perviousView, s.location, s.stepPlan, s.stepIndex]),
  );

  const isEditingDefaults = perviousView === null;
  const mfaSteps = isPresent(location) ? mfaStepsOf(location) : [];
  const isMultiStep = isPresent(location) && mfaStepCount(location) > 1;
  const defaultPlan = isPresent(location) ? resolveMfaStepPlan(location) : [];

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
    if (!isPresent(location)) return;

    if (isEditingDefaults) {
      setMfaStepPlan({ locationId: location.id, mfaStepPlan: selectedStepMethods });
      useConnectModal.setState({ visible: false });
      return;
    }

    if (!isMultiStep && saveAsDefault) {
      setMfaStepPlan({ locationId: location.id, mfaStepPlan: selectedStepMethods });
    }

    const methodForCurrentStep = selectedStepMethods[stepIndex];
    useConnectModal
      .getState()
      .setView(mfaMethodToConnectModalView(methodForCurrentStep), {
        stepPlan: selectedStepMethods,
        mfaMethod: methodForCurrentStep,
      });
  };

  return (
    <div id="mfa-settings-view">
      {!isEditingDefaults && (
        <p className="view-description">
          {isMultiStep
            ? `If you're having issues with your current verification method, you can choose another one for this login.`
            : `If you're having issues with your current verification method, you can choose another one or set a new default.`}
        </p>
      )}
      {isEditingDefaults && (
        <p className="view-description">
          {`Choose the default verification method for each step of this location.`}
        </p>
      )}
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
        <Fragment>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Checkbox
            active={saveAsDefault}
            onClick={() => setSaveAsDefault((current) => !current)}
            text="Set as default MFA method"
          />
        </Fragment>
      )}
      <SizedBox height={isEditingDefaults ? ThemeSpacing.Xl3 : ThemeSpacing.Xl2} />
      <Controls>
        {!isEditingDefaults && (
          <Button
            variant={ButtonVariant.Secondary}
            text="Cancel"
            onClick={() => useConnectModal.getState().setView(perviousView)}
          />
        )}
        <div className="right">
          <Button
            variant={ButtonVariant.Primary}
            size="primary"
            text={isEditingDefaults ? 'Save changes' : 'Continue'}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
