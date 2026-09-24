import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { Fragment, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { Button } from '../../../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../../../shared/components/Button/types';
import { Checkbox } from '../../../../../../../shared/components/Checkbox/Checkbox';
import { Controls } from '../../../../../../../shared/components/Controls/Controls';
import { MfaSettingsSection } from '../../../../../../../shared/components/MfaSettingsSection/MfaSettingsSection';
import { useMfaSettingsSection } from '../../../../../../../shared/components/MfaSettingsSection/useMfaSettingsSection';
import { SizedBox } from '../../../../../../../shared/components/SizedBox/SizedBox';
import { useConfigureFactorsScreen } from '../../../../../../../shared/hooks/useConfigureFactorsScreen';
import { useAppData } from '../../../../../../../shared/providers/AppDataContext';
import { api } from '../../../../../../../shared/rust-api/api';
import type { LocationInfo } from '../../../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../../../shared/types';
import { mfaEditConfigureFactorsSource } from '../../../../../../../shared/utils/configureFactorsSource';
import { isPresent } from '../../../../../../../shared/utils/isPresent';
import { mfaStepCount } from '../../../../../../../shared/utils/mfa';
import { mfaMethodToConnectModalView } from '../../hooks/types';
import { useConnectModal } from '../../hooks/useConnectModal';

export const ConnectModalMfaSettings = () => {
  const location = useConnectModal((s) => s.location);
  if (!isPresent(location)) return null;
  return <ConnectModalMfaSettingsContent key={location.id} location={location} />;
};

const ConnectModalMfaSettingsContent = ({ location }: { location: LocationInfo }) => {
  const { mutate: setMfaStepPlan } = useMutation({
    mutationFn: api.setLocationMfaStepPlan,
    meta: { invalidate: [['locations']] },
  });

  const [perviousView, stepPlan, stepIndex] = useConnectModal(
    useShallow((s) => [s.perviousView, s.stepPlan, s.stepIndex]),
  );

  const { instances } = useAppData();
  const instance = instances.find((entry) => entry.id === location.instance_id);

  const isEditingDefaults = perviousView === null;
  const isMultiStep = mfaStepCount(location) > 1;

  const mfaSection = useMfaSettingsSection({
    location,
    instance,
    initialPlan: isEditingDefaults ? undefined : stepPlan,
    stepIndices: isEditingDefaults ? undefined : [stepIndex],
    // Configuring would abandon the connect attempt.
    configurable: isEditingDefaults,
  });
  const [saveAsDefault, setSaveAsDefault] = useState(false);

  const { mutate: configureFactors, isPending: isOpeningConfiguration } =
    useConfigureFactorsScreen({
      onSuccess: () => useConnectModal.setState({ visible: false }),
    });

  const configureCount = mfaSection.configureMethods.length;
  const isConfiguring = isEditingDefaults && configureCount > 0;

  const handleSubmit = () => {
    if (isEditingDefaults) {
      setMfaStepPlan({ locationId: location.id, mfaStepPlan: mfaSection.plan });
      if (isConfiguring) {
        configureFactors({
          instanceId: location.instance_id,
          methods: mfaSection.configureMethods,
          source: mfaEditConfigureFactorsSource(),
          locationId: location.id,
        });
        return;
      }
      useConnectModal.setState({ visible: false });
      return;
    }

    if (!isMultiStep && saveAsDefault) {
      setMfaStepPlan({ locationId: location.id, mfaStepPlan: mfaSection.plan });
    }

    const methodForCurrentStep = mfaSection.plan[stepIndex];
    useConnectModal
      .getState()
      .setView(mfaMethodToConnectModalView(methodForCurrentStep), {
        stepPlan: mfaSection.plan,
        mfaMethod: methodForCurrentStep,
      });
  };

  let submitText = isEditingDefaults ? 'Save changes' : 'Continue';
  if (isConfiguring) submitText = `Configure MFA (${configureCount})`;

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
      <MfaSettingsSection {...mfaSection.sectionProps} />
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
            text={submitText}
            disabled={isOpeningConfiguration}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
