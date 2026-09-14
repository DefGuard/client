import { error as logError } from '@tauri-apps/plugin-log';
import dayjs from 'dayjs';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { api } from '../../../../shared/rust-api/api';
import {
  type InstanceInfo,
  type MfaConfigAuthorizeResult,
  type MfaConfigStartResult,
  MfaMethod,
  type MfaMethodValue,
} from '../../../../shared/rust-api/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import {
  ConfigureMfaStep,
  type ConfigureMfaStepValue,
  isCodeMfaMethod,
  isMfaSetupStep,
  MFA_WIZARD_STEPS,
  mfaFactorStep,
} from '../types';

type StoreValues = {
  activeStep: ConfigureMfaStepValue;
  instance: InstanceInfo | null;
  sessionId: string | null;
  /** Code factors come from the session, the rest from the instance snapshot. */
  configuredMethods: MfaMethodValue[];
  /** Null until the selection step is done. Empty is valid, the email fallback configures
   *  a factor on its own. */
  selectedMethods: MfaMethodValue[] | null;
  /** No factor was configured, so an emailed code was the only way in. */
  emailFallback: boolean;
  /** ISO timestamp the session dies at, null once nothing in the flow needs it. */
  deadline: string | null;
  authorized: boolean;
  /** Issued for the account's first factor only, so an empty list is an ordinary success. */
  recoveryCodes: string[];
};

type FlowState = Pick<
  StoreValues,
  'configuredMethods' | 'selectedMethods' | 'recoveryCodes'
>;

/** Picked factors still to set up, in wizard order. */
const pendingMethods = (state: FlowState): MfaMethodValue[] =>
  state.selectedMethods?.filter((method) => !state.configuredMethods.includes(method)) ??
  [];

/** Setup steps with a factor still pending, plus the closing steps that have something to show. */
const remainingSteps = (state: FlowState): ConfigureMfaStepValue[] => {
  const pending = pendingMethods(state);
  return MFA_WIZARD_STEPS.filter((step) => {
    switch (step) {
      case ConfigureMfaStep.RecoveryCodes:
        return state.recoveryCodes.length > 0;
      case ConfigureMfaStep.Finish:
        return true;
      default:
        return pending.some((method) => mfaFactorStep(method) === step);
    }
  });
};

/** Picking only the email fallback leaves no setup step, so the wizard opens on what follows. */
const firstStep = (state: FlowState): ConfigureMfaStepValue =>
  remainingSteps(state)[0] ?? ConfigureMfaStep.Finish;

const defaults: StoreValues = {
  activeStep: ConfigureMfaStep.Configuration,
  instance: null,
  sessionId: null,
  configuredMethods: [],
  selectedMethods: null,
  emailFallback: false,
  deadline: null,
  authorized: false,
  recoveryCodes: [],
};

interface Store extends StoreValues {
  start: (instance: InstanceInfo, response: MfaConfigStartResult) => void;
  selectMethods: (methods: MfaMethodValue[]) => void;
  /** The fresh deadline bounds every setup in the session, not just the next one. */
  authorize: (response: MfaConfigAuthorizeResult) => void;
  factorConfigured: (method: MfaMethodValue, recoveryCodes: string[]) => void;
  next: () => void;
  back: () => void;
  reset: () => void;
}

export const useConfigureMfaStore = create<Store>()(
  persist(
    (set, get) => ({
      ...defaults,
      start: (instance, response) => {
        // The fallback mails a code to the address on file, registering email along the way.
        const codeFactors = response.email_fallback
          ? [MfaMethod.Email]
          : response.available_methods;
        const configuredMethods = [
          ...codeFactors,
          ...(instance.mfa_configured_methods ?? []).filter(
            (method) => !isCodeMfaMethod(method),
          ),
        ];
        set({
          ...defaults,
          instance,
          sessionId: response.session_id,
          configuredMethods,
          emailFallback: response.email_fallback,
          deadline: dayjs.unix(response.deadline_timestamp).toISOString(),
        });
      },
      selectMethods: (methods) => {
        set((current) => ({
          selectedMethods: methods,
          activeStep: firstStep({ ...current, selectedMethods: methods }),
        }));
      },
      authorize: (response) => {
        set((current) => ({
          authorized: true,
          deadline: dayjs.unix(response.deadline_timestamp).toISOString(),
          // The fallback enables email as it verifies, so only this authorization issues codes.
          recoveryCodes: response.recovery_codes,
          activeStep: firstStep({ ...current, recoveryCodes: response.recovery_codes }),
        }));
      },
      factorConfigured: (method, recoveryCodes) => {
        set((current) => ({
          configuredMethods: current.configuredMethods.includes(method)
            ? current.configuredMethods
            : [...current.configuredMethods, method],
          recoveryCodes: recoveryCodes.length > 0 ? recoveryCodes : current.recoveryCodes,
          // Last step that needs the session, so nothing is left to expire.
          deadline: method === MfaMethod.Fido2 ? null : current.deadline,
        }));
      },
      next: () => {
        const current = get();
        const from = MFA_WIZARD_STEPS.indexOf(current.activeStep);
        // A setup step holds the flow while it still has a picked factor to configure.
        const next = remainingSteps(current).find(
          (step) =>
            MFA_WIZARD_STEPS.indexOf(step) > from ||
            (step === current.activeStep && isMfaSetupStep(step)),
        );
        if (isPresent(next)) set({ activeStep: next });
      },
      back: () => {
        const current = get();
        const from = MFA_WIZARD_STEPS.indexOf(current.activeStep);
        // A step with nothing left to do is not one to go back to.
        const previous = remainingSteps(current)
          .filter((step) => MFA_WIZARD_STEPS.indexOf(step) < from)
          .at(-1);
        if (isPresent(previous)) set({ activeStep: previous });
      },
      reset: () => {
        set({ ...defaults });
      },
    }),
    {
      name: 'configure-mfa-store',
      storage: createJSONStorage(() => sessionStorage),
      // Bumped when the picks became one list, so older sessions resume with no selection.
      version: 6,
    },
  ),
);

/** The factor a setup step is currently working on. */
export const selectPendingMethod =
  (step: ConfigureMfaStepValue) =>
  (state: Store): MfaMethodValue | undefined =>
    pendingMethods(state).find((method) => mfaFactorStep(method) === step);

export const startMfaConfiguration = async (instance: InstanceInfo): Promise<void> => {
  const response = await api.mfaConfigStart(instance.id);
  useConfigureMfaStore.getState().start(instance, response);
};

/** A copy the proxy still holds expires on its own, so a failed cancel is not worth raising. */
export const discardMfaConfiguration = async (): Promise<void> => {
  const { sessionId } = useConfigureMfaStore.getState();
  useConfigureMfaStore.getState().reset();
  if (!isPresent(sessionId)) return;
  try {
    await api.mfaConfigCancel(sessionId);
  } catch (err) {
    void logError(`Failed to cancel MFA configuration session: ${err}`);
  }
};
