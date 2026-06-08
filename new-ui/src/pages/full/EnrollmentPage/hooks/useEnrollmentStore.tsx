import dayjs from 'dayjs';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type {
  EnrollmentStartResponse,
  MfaMethodValue,
} from '../../../../shared/rust-api/types';
import { EnrollmentStep, type EnrollmentStepValue } from '../types';

type StoreValues = {
  activeStep: EnrollmentStepValue;
  startResponse: EnrollmentStartResponse | null;
  proxyUrl: string | null;
  skipMfa: boolean;
  skipPassword: boolean;
  skipMfaChoice: boolean;
  deadline: string | null;
  userPassword: string | null;
  userTotpSecret: string | null;
  userRecoveryCodes: string[] | null;
  userMfaChoice: MfaMethodValue;
};

const defaults: StoreValues = {
  activeStep: EnrollmentStep.Welcome,
  proxyUrl: null,
  userRecoveryCodes: null,
  startResponse: null,
  skipMfa: false,
  skipPassword: false,
  skipMfaChoice: true,
  deadline: null,
  userPassword: null,
  userTotpSecret: null,
  userMfaChoice: 'totp',
} as const;

interface Store extends StoreValues {
  start: (response: EnrollmentStartResponse, url: string, totpSecret?: string) => void;
  next: () => void;
  back: () => void;
}

export const useEnrollmentStore = create<Store>()(
  persist(
    (set, get) => ({
      ...defaults,
      start: (response, url, secret) => {
        set({
          ...defaults,
          proxyUrl: url,
          activeStep: EnrollmentStep.Welcome,
          startResponse: response,
          // if smtp is not present then it's not possible to choose.
          skipMfaChoice: !response.settings.smtp_configured,
          deadline: dayjs.unix(response.deadline_timestamp).toISOString(),
          userTotpSecret: secret ?? null,
        });
      },
      back: () => {
        const { activeStep, skipPassword, skipMfa, skipMfaChoice } = get();
        let prevStep: EnrollmentStepValue;
        switch (activeStep) {
          case EnrollmentStep.Welcome:
            return;
          case EnrollmentStep.Password:
            prevStep = EnrollmentStep.Welcome;
            break;
          case EnrollmentStep.MfaChoice:
            prevStep = skipPassword ? EnrollmentStep.Welcome : EnrollmentStep.Password;
            break;
          case EnrollmentStep.MfaConfiguration:
            if (!skipMfaChoice) {
              prevStep = EnrollmentStep.MfaChoice;
            } else if (!skipPassword) {
              prevStep = EnrollmentStep.Password;
            } else {
              prevStep = EnrollmentStep.Welcome;
            }
            break;
          case EnrollmentStep.RecoveryCodes:
            prevStep = EnrollmentStep.MfaConfiguration;
            break;
          case EnrollmentStep.Finish:
            if (!skipMfa) {
              prevStep = EnrollmentStep.RecoveryCodes;
            } else if (!skipPassword) {
              prevStep = EnrollmentStep.Password;
            } else {
              prevStep = EnrollmentStep.Welcome;
            }
            break;
          default:
            return;
        }
        set({ activeStep: prevStep });
      },
      next: () => {
        const { activeStep, skipPassword, skipMfa, skipMfaChoice } = get();
        let nextStep: EnrollmentStepValue;
        switch (activeStep) {
          case EnrollmentStep.Welcome:
            if (!skipPassword) {
              nextStep = EnrollmentStep.Password;
            } else if (!skipMfaChoice && !skipMfa) {
              nextStep = EnrollmentStep.MfaChoice;
            } else if (!skipMfa) {
              nextStep = EnrollmentStep.MfaConfiguration;
            } else {
              nextStep = EnrollmentStep.Finish;
            }
            break;
          case EnrollmentStep.Password:
            if (!skipMfaChoice && !skipMfa) {
              nextStep = EnrollmentStep.MfaChoice;
            } else if (!skipMfa) {
              nextStep = EnrollmentStep.MfaConfiguration;
            } else {
              nextStep = EnrollmentStep.Finish;
            }
            break;
          case EnrollmentStep.MfaChoice:
            nextStep = EnrollmentStep.MfaConfiguration;
            break;
          case EnrollmentStep.MfaConfiguration:
            nextStep = EnrollmentStep.RecoveryCodes;
            break;
          case EnrollmentStep.RecoveryCodes:
            nextStep = EnrollmentStep.Finish;
            break;
          case EnrollmentStep.Finish:
            return;
          default:
            return;
        }
        set({ activeStep: nextStep });
      },
    }),
    {
      name: 'enrollment-store',
      storage: createJSONStorage(() => sessionStorage),
      version: 1,
    },
  ),
);
