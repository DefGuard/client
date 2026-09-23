import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';
import { useConfigureFactorsScreen } from '../../../../hooks/useConfigureFactorsScreen';
import { api } from '../../../../rust-api/api';
import { ThemeSpacing } from '../../../../types';
import { mfaEditConfigureFactorsSource } from '../../../../utils/configureFactorsSource';
import { mfaStepCount } from '../../../../utils/mfa';
import { Button } from '../../../Button/Button';
import { ButtonVariant } from '../../../Button/types';
import { Checkbox } from '../../../Checkbox/Checkbox';
import { Controls } from '../../../Controls/Controls';
import { Divider } from '../../../Divider/Divider';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { MfaSettingsSection } from '../../../MfaSettingsSection/MfaSettingsSection';
import { useMfaSettingsSection } from '../../../MfaSettingsSection/useMfaSettingsSection';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { LocationViewHeader } from '../../components/LocationViewHeader/LocationViewHeader';
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
    instance,
    setMfaMethod: setContextMethod,
    stepPlan,
    stepIndex,
    setStepPlanOnce,
  } = useLocationCardContext();

  const isMultiStep = mfaStepCount(location) > 1;

  const isEditingDefaults = previousView === LocationCardViews.Default;

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
      onSuccess: () => setView(LocationCardViews.Default),
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
      setView(LocationCardViews.Default);
      return;
    }

    if (!isMultiStep && saveAsDefault) {
      setMfaStepPlan({ locationId: location.id, mfaStepPlan: mfaSection.plan });
    }

    const methodForCurrentStep = mfaSection.plan[stepIndex];
    setStepPlanOnce(mfaSection.plan);
    setContextMethod(methodForCurrentStep);
    setView(mfaMethodToLocationCardView(methodForCurrentStep));
  };

  let submitText = isEditingDefaults ? 'Save changes' : 'Continue';
  if (isConfiguring) submitText = `Configure MFA (${configureCount})`;

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
      <MfaSettingsSection {...mfaSection.sectionProps} />
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
            text={submitText}
            disabled={isOpeningConfiguration}
            onClick={handleSubmit}
          />
        </div>
      </Controls>
    </div>
  );
};
