import type { ReactNode } from 'react';
import type { WizardPageStep } from '../../../shared/components/wizard/types';
import { WizardPage } from '../../../shared/components/wizard/WizardPage/WizardPage';
import { useTunnelWizardStore } from './hooks/useTunnelWizardStore';
import { AdvancedSettingsStep } from './steps/AdvancedSettingsStep/AdvancedSettingsStep';
import { FinishStep } from './steps/FinishStep/FinishStep';
import { GeneralInformationStep } from './steps/GeneralInformationStep/GeneralInformationStep';
import { KeysStep } from './steps/KeysStep/KeysStep';
import { VpnServerStep } from './steps/VpnServerStep/VpnServerStep';
import { TunnelWizardStep, type TunnelWizardStepValue } from './types';

const stepComponents: Record<TunnelWizardStepValue, ReactNode> = {
  [TunnelWizardStep.GeneralInformation]: <GeneralInformationStep />,
  [TunnelWizardStep.Keys]: <KeysStep />,
  [TunnelWizardStep.VpnServer]: <VpnServerStep />,
  [TunnelWizardStep.AdvancedSettings]: <AdvancedSettingsStep />,
  [TunnelWizardStep.Finish]: <FinishStep />,
};

export const TunnelWizardPage = () => {
  const { activeStep } = useTunnelWizardStore();

  const steps: Record<TunnelWizardStepValue, WizardPageStep> = {
    [TunnelWizardStep.GeneralInformation]: {
      id: TunnelWizardStep.GeneralInformation,
      order: 1,
      label: 'General Information',
    },
    [TunnelWizardStep.Keys]: {
      id: TunnelWizardStep.Keys,
      order: 2,
      label: 'Keys',
    },
    [TunnelWizardStep.VpnServer]: {
      id: TunnelWizardStep.VpnServer,
      order: 3,
      label: 'VPN Server',
    },
    [TunnelWizardStep.AdvancedSettings]: {
      id: TunnelWizardStep.AdvancedSettings,
      order: 4,
      label: 'Advanced Settings',
    },
    [TunnelWizardStep.Finish]: {
      id: TunnelWizardStep.Finish,
      order: 5,
      label: 'Finish',
    },
  };

  return (
    <WizardPage
      title="Add WireGuard Tunnel"
      subtitle=""
      activeStep={activeStep}
      steps={steps}
    >
      {stepComponents[activeStep]}
    </WizardPage>
  );
};
