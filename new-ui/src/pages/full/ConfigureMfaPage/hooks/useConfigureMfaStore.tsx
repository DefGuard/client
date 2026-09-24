import { error as logError } from '@tauri-apps/plugin-log';
import dayjs from 'dayjs';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { api } from '../../../../shared/rust-api/api';
import {
  type ConfigureFactorsSourceValue,
  type InstanceInfo,
  type LocationInfo,
  type MfaConfigAuthorizeResult,
  type MfaConfigStartResult,
  MfaMethod,
  type MfaMethodValue,
} from '../../../../shared/rust-api/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import {
  ConfigureMfaStep,
  type ConfigureMfaStepValue,
  MFA_WIZARD_STEPS,
  type MfaVerificationMethod,
} from '../types';
import {
  isCodeMfaMethod,
  isMfaFactorOfferable,
  isMfaSetupStep,
  mfaFactorStep,
} from '../utils';

type StoreValues = {
  activeStep: ConfigureMfaStepValue;
  instance: InstanceInfo | null;
  sessionId: string | null;
  /** Snapshot taken at the start of the session and never moved, or a repeatable factor would
   *  look configured before it was offered. */
  configuredMethods: MfaMethodValue[];
  /** Factors this run has set up, which is what the wizard steps through. */
  completedMethods: MfaMethodValue[];
  /** Null until the selection step is done. Empty is valid, the email fallback configures
   *  a factor on its own. */
  selectedMethods: MfaMethodValue[] | null;
  /** Pre-ticked in the selection step, from the entry point or an earlier pass. */
  initialSelection: MfaMethodValue[];
  /** Null until picked. Asked only when more than one code factor is configured. */
  verificationMethod: MfaVerificationMethod | null;
  /** No factor was configured, so an emailed code was the only way in. */
  emailFallback: boolean;
  /** ISO timestamp the session dies at, null once nothing in the flow needs it. */
  deadline: string | null;
  authorized: boolean;
  /** Issued for the account's first factor only, so an empty list is an ordinary success. */
  recoveryCodes: string[];
  /** Which entry point opened this flow. Recorded for later, nothing branches on it yet. */
  source: ConfigureFactorsSourceValue | null;
  /** The location that sent the user here, so the screens can speak to what that location
   *  needs. Null when the flow was not started from a location. */
  location: LocationInfo | null;
};

type FlowState = Pick<
  StoreValues,
  'configuredMethods' | 'completedMethods' | 'selectedMethods' | 'recoveryCodes'
>;

/** Picked factors still to set up, in wizard order. */
const pendingMethods = (state: FlowState): MfaMethodValue[] =>
  state.selectedMethods?.filter(
    (method) =>
      !state.completedMethods.includes(method) &&
      // Guards against a pick the selection screen should already have refused.
      isMfaFactorOfferable(method, state.configuredMethods),
  ) ?? [];

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

/** The session is only needed while a setup is still to come. Letting it run past the last one
 *  would expire the flow under a user still reading their recovery codes. */
const sessionDeadline = (state: FlowState, deadline: string | null): string | null =>
  pendingMethods(state).length > 0 ? deadline : null;

const defaults: StoreValues = {
  activeStep: ConfigureMfaStep.Configuration,
  instance: null,
  sessionId: null,
  configuredMethods: [],
  completedMethods: [],
  selectedMethods: null,
  initialSelection: [],
  verificationMethod: null,
  emailFallback: false,
  deadline: null,
  authorized: false,
  recoveryCodes: [],
  source: null,
  location: null,
};

/** What the entry point knew about the flow it is opening. */
type ConfigureMfaOrigin = Pick<StoreValues, 'source' | 'location'>;

interface Store extends StoreValues {
  start: (
    instance: InstanceInfo,
    response: MfaConfigStartResult,
    origin: ConfigureMfaOrigin,
  ) => void;
  selectMethods: (methods: MfaMethodValue[]) => void;
  selectVerificationMethod: (method: MfaVerificationMethod) => void;
  /** Keeps the current picks ticked and the session alive. */
  backToSelection: () => void;
  /** The fresh deadline bounds every setup still to come, not just the next one. */
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
      start: (instance, response, origin) => {
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
          ...origin,
        });
      },
      selectMethods: (methods) => {
        set((current) => ({
          selectedMethods: methods,
          activeStep: firstStep({ ...current, selectedMethods: methods }),
        }));
      },
      selectVerificationMethod: (method) => {
        set({ verificationMethod: method });
      },
      backToSelection: () => {
        set((current) => ({
          initialSelection: current.selectedMethods ?? current.initialSelection,
          selectedMethods: null,
          verificationMethod: null,
          activeStep: defaults.activeStep,
        }));
      },
      authorize: (response) => {
        set((current) => {
          // The fallback enables email as it verifies, so only this authorization issues codes.
          const next = { ...current, recoveryCodes: response.recovery_codes };
          return {
            authorized: true,
            recoveryCodes: response.recovery_codes,
            deadline: sessionDeadline(
              next,
              dayjs.unix(response.deadline_timestamp).toISOString(),
            ),
            activeStep: firstStep(next),
          };
        });
      },
      factorConfigured: (method, recoveryCodes) => {
        set((current) => {
          const completedMethods = current.completedMethods.includes(method)
            ? current.completedMethods
            : [...current.completedMethods, method];
          const codes = recoveryCodes.length > 0 ? recoveryCodes : current.recoveryCodes;
          return {
            completedMethods,
            recoveryCodes: codes,
            deadline: sessionDeadline(
              { ...current, completedMethods, recoveryCodes: codes },
              current.deadline,
            ),
          };
        });
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
      // Bumped on every shape change: a stored session is never resumable across one.
      version: 11,
    },
  ),
);

/** The factor a setup step is currently working on. */
export const selectPendingMethod =
  (step: ConfigureMfaStepValue) =>
  (state: Store): MfaMethodValue | undefined =>
    pendingMethods(state).find((method) => mfaFactorStep(method) === step);

type StartOptions = Partial<ConfigureMfaOrigin> & {
  preselectedMethods?: MfaMethodValue[];
};

export const startMfaConfiguration = async (
  instance: InstanceInfo,
  { preselectedMethods = [], source = null, location = null }: StartOptions = {},
): Promise<void> => {
  const response = await api.mfaConfigStart(instance.id);
  useConfigureMfaStore.getState().start(instance, response, { source, location });
  // Only pre-ticks, the user still confirms in the selection step.
  const initialSelection = preselectedMethods.filter((method) =>
    isMfaFactorOfferable(method, useConfigureMfaStore.getState().configuredMethods),
  );
  useConfigureMfaStore.setState({ initialSelection });
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
