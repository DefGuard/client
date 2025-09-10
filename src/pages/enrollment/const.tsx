import { cloneDeep } from 'lodash-es';
import type { ReactNode } from 'react';
import { ChooseMfaStep } from './steps/ChooseMfaStep/ChooseMfaStep';
import { DataVerificationStep } from './steps/DataVerificationStep/DataVerificationStep';
import { DeviceStep } from './steps/DeviceStep/DeviceStep';
import { FinishStep } from './steps/FinishStep/FinishStep';
import { MfaRecoveryCodesStep } from './steps/MfaRecoveryCodesStep/MfaRecoveryCodesStep';
import { MfaSetupStep } from './steps/MfaSetupStep/MfaSetupStep';
import { PasswordStep } from './steps/PasswordStep/PasswordStep';
import { SendFinishStep } from './steps/SendFinishStep/SendFinishStep';
import { WelcomeStep } from './steps/WelcomeStep/WelcomeStep';

export enum EnrollmentStepKey {
  WELCOME = 'welcome',
  DATA_VERIFICATION = 'data-verification',
  PASSWORD = 'password',
  DEVICE = 'device',
  MFA = 'mfa',
  MFA_CHOICE = 'mfa-choice',
  MFA_SETUP = 'mfa-setup',
  MFA_RECOVERY = 'mfa-recovery',
  ACTIVATE_USER = 'activate',
  FINISH = 'finish',
}

export type EnrollmentStep = {
  key: EnrollmentStepKey;
  sideBarPrefix?: string;
  indicatorPrefix?: string;
  // enable back in navigation
  backEnabled?: boolean;
  nextDisabled?: boolean;
  children?: EnrollmentStep[];
  // this means it's only rendered and it doesn't count as a step in UI
  // meant for loading in-between steps like send finish
  hidden?: boolean;
};

// this servers as configuration for side bar and steps indicator
// some steps are like in between loaders and mfa has sub steps that's why this needs to exist
// in side bar this serves as base for final config with translated labels
export const enrollmentStepsConfig: Record<string, EnrollmentStep> = {
  [EnrollmentStepKey.WELCOME]: {
    key: EnrollmentStepKey.WELCOME,
    sideBarPrefix: '1',
  },
  [EnrollmentStepKey.DATA_VERIFICATION]: {
    key: EnrollmentStepKey.DATA_VERIFICATION,
    sideBarPrefix: '2',
  },
  [EnrollmentStepKey.PASSWORD]: {
    key: EnrollmentStepKey.PASSWORD,
    sideBarPrefix: '3',
    backEnabled: true,
  },
  [EnrollmentStepKey.DEVICE]: {
    key: EnrollmentStepKey.DEVICE,
    sideBarPrefix: '4',
    backEnabled: true,
  },
  [EnrollmentStepKey.MFA]: {
    key: EnrollmentStepKey.MFA,
    sideBarPrefix: '5',
    children: [
      {
        key: EnrollmentStepKey.MFA_CHOICE,
        sideBarPrefix: 'a',
        indicatorPrefix: '5a',
        nextDisabled: true,
      },
      {
        key: EnrollmentStepKey.MFA_SETUP,
        sideBarPrefix: 'b',
        indicatorPrefix: '5b',
        backEnabled: true,
      },
      {
        key: EnrollmentStepKey.MFA_RECOVERY,
        sideBarPrefix: 'c',
        indicatorPrefix: '5c',
      },
    ],
  },
  [EnrollmentStepKey.ACTIVATE_USER]: {
    key: EnrollmentStepKey.ACTIVATE_USER,
    hidden: true,
    nextDisabled: true,
  },
  [EnrollmentStepKey.FINISH]: {
    sideBarPrefix: '6',
    key: EnrollmentStepKey.FINISH,
  },
};

export const enrollmentSteps: Record<EnrollmentStepKey, ReactNode | null> = {
  [EnrollmentStepKey.WELCOME]: <WelcomeStep />,
  [EnrollmentStepKey.DATA_VERIFICATION]: <DataVerificationStep />,
  [EnrollmentStepKey.PASSWORD]: <PasswordStep />,
  [EnrollmentStepKey.DEVICE]: <DeviceStep />,
  // this will be skipped and is here only for TS
  [EnrollmentStepKey.MFA]: null,
  [EnrollmentStepKey.MFA_RECOVERY]: <MfaRecoveryCodesStep />,
  [EnrollmentStepKey.MFA_CHOICE]: <ChooseMfaStep />,
  [EnrollmentStepKey.MFA_SETUP]: <MfaSetupStep />,
  [EnrollmentStepKey.ACTIVATE_USER]: <SendFinishStep />,
  [EnrollmentStepKey.FINISH]: <FinishStep />,
};

export const flattenEnrollConf = () => {
  const steps = cloneDeep(enrollmentStepsConfig);
  Object.values(steps).forEach((step) => {
    if (step.children) {
      step.children.forEach((child) => {
        steps[child.key] = child;
      });
    }
  });
  return steps;
};
