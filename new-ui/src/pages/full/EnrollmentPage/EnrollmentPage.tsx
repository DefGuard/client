import { type ReactNode, useMemo } from 'react';
import type { WizardPageStep } from '../../../shared/components/wizard/types';
import { WizardPage } from '../../../shared/components/wizard/WizardPage/WizardPage';
import { EnrollmentTimeoutProvider } from './components/EnrollmentTimeoutProvider';
import { useEnrollmentStore } from './hooks/useEnrollmentStore';
import { FinishStep } from './steps/FinishStep/FinishStep';
import { MfaChoiceStep } from './steps/MfaChoiceStep/MfaChoiceStep';
import { MfaConfigurationStep } from './steps/MfaConfigurationStep/MfaConfigurationStep';
import { PasswordStep } from './steps/PasswordStep/PasswordStep';
import { RecoveryCodesStep } from './steps/RecoveryCodesStep/RecoveryCodesStep';
import { WelcomeStep } from './steps/WelcomeStep/WelcomeStep';
import { EnrollmentStep, type EnrollmentStepValue } from './types';

const stepComponents: Record<EnrollmentStepValue, ReactNode> = {
  [EnrollmentStep.Welcome]: <WelcomeStep />,
  [EnrollmentStep.Password]: <PasswordStep />,
  [EnrollmentStep.MfaChoice]: <MfaChoiceStep />,
  [EnrollmentStep.MfaConfiguration]: <MfaConfigurationStep />,
  [EnrollmentStep.RecoveryCodes]: <RecoveryCodesStep />,
  [EnrollmentStep.Finish]: <FinishStep />,
};

export const EnrollmentPage = () => {
  const { activeStep, skipPassword, skipMfa, skipMfaChoice } = useEnrollmentStore();

  const steps = useMemo(
    (): Record<EnrollmentStepValue, WizardPageStep> => ({
      [EnrollmentStep.Welcome]: {
        id: EnrollmentStep.Welcome,
        order: 1,
        label: 'Welcome',
      },
      [EnrollmentStep.Password]: {
        id: EnrollmentStep.Password,
        order: 2,
        label: 'Password',
        hidden: skipPassword,
      },
      [EnrollmentStep.MfaChoice]: {
        id: EnrollmentStep.MfaChoice,
        order: 3,
        label: 'MFA Method',
        hidden: skipMfaChoice || skipMfa,
      },
      [EnrollmentStep.MfaConfiguration]: {
        id: EnrollmentStep.MfaConfiguration,
        order: 4,
        label: 'MFA Configuration',
        hidden: skipMfa,
      },
      [EnrollmentStep.RecoveryCodes]: {
        id: EnrollmentStep.RecoveryCodes,
        order: 5,
        label: 'Recovery Codes',
        hidden: skipMfa,
      },
      [EnrollmentStep.Finish]: {
        id: EnrollmentStep.Finish,
        order: 6,
        label: 'Finish',
      },
    }),
    [skipMfa, skipMfaChoice, skipPassword],
  );

  return (
    <EnrollmentTimeoutProvider>
      <WizardPage title="Enrollment" subtitle="" activeStep={activeStep} steps={steps}>
        {stepComponents[activeStep]}
      </WizardPage>
    </EnrollmentTimeoutProvider>
  );
};
