import { type ReactNode, useMemo } from 'react';
import type { WizardPageStep } from '../../../../shared/components/wizard/types';
import { WizardPage } from '../../../../shared/components/wizard/WizardPage/WizardPage';
import { useConfigureMfaStore } from '../hooks/useConfigureMfaStore';
import { ConfigureMfaStep, type ConfigureMfaStepValue } from '../types';
import { mfaStepsOf } from '../utils';
import { ConfigureFactorStep } from './ConfigureFactorStep/ConfigureFactorStep';
import { ConfigureFido2Step } from './ConfigureFido2Step/ConfigureFido2Step';
import { ConfigureFinishStep } from './ConfigureFinishStep/ConfigureFinishStep';
import { ConfigureRecoveryCodesStep } from './ConfigureRecoveryCodesStep/ConfigureRecoveryCodesStep';

type Props = {
  onCancel: () => void;
  onSessionExpired: () => void;
};

export const ConfigureMfaWizard = ({ onCancel, onSessionExpired }: Props) => {
  const activeStep = useConfigureMfaStore((s) => s.activeStep);
  const hasRecoveryCodes = useConfigureMfaStore((s) => s.recoveryCodes.length > 0);
  const selectedMethods = useConfigureMfaStore((s) => s.selectedMethods);

  // The picks, not what is left of them, so a finished step keeps its place in the indicator.
  const selectedSteps = useMemo(
    () => mfaStepsOf(selectedMethods ?? []),
    [selectedMethods],
  );

  const steps = useMemo(
    (): Record<ConfigureMfaStepValue, WizardPageStep> => ({
      [ConfigureMfaStep.Configuration]: {
        id: ConfigureMfaStep.Configuration,
        order: 1,
        label: 'MFA Configuration',
        hidden: !selectedSteps.includes(ConfigureMfaStep.Configuration),
      },
      [ConfigureMfaStep.Fido2]: {
        id: ConfigureMfaStep.Fido2,
        order: 2,
        label: 'Security Key',
        hidden: !selectedSteps.includes(ConfigureMfaStep.Fido2),
      },
      [ConfigureMfaStep.RecoveryCodes]: {
        id: ConfigureMfaStep.RecoveryCodes,
        order: 3,
        label: 'Recovery Codes',
        hidden: !hasRecoveryCodes,
      },
      [ConfigureMfaStep.Finish]: {
        id: ConfigureMfaStep.Finish,
        order: 4,
        label: 'Finish',
      },
    }),
    [hasRecoveryCodes, selectedSteps],
  );

  const stepComponents = useMemo(
    (): Record<ConfigureMfaStepValue, ReactNode> => ({
      [ConfigureMfaStep.Configuration]: (
        <ConfigureFactorStep onCancel={onCancel} onSessionExpired={onSessionExpired} />
      ),
      [ConfigureMfaStep.Fido2]: (
        <ConfigureFido2Step onCancel={onCancel} onSessionExpired={onSessionExpired} />
      ),
      [ConfigureMfaStep.RecoveryCodes]: <ConfigureRecoveryCodesStep />,
      [ConfigureMfaStep.Finish]: <ConfigureFinishStep />,
    }),
    [onCancel, onSessionExpired],
  );

  return (
    <WizardPage title="Configure MFA" subtitle="" activeStep={activeStep} steps={steps}>
      {stepComponents[activeStep]}
    </WizardPage>
  );
};
