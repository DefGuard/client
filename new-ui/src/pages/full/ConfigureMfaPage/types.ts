import { MfaMethod, type MfaMethodValue } from '../../../shared/rust-api/types';

/** Wizard steps, in the order they run. Setup steps are the ones a factor can claim. */
export const ConfigureMfaStep = {
  Configuration: 'configuration',
  Fido2: 'fido2',
  RecoveryCodes: 'recovery-codes',
  Finish: 'finish',
} as const;

export type ConfigureMfaStepValue =
  (typeof ConfigureMfaStep)[keyof typeof ConfigureMfaStep];

export const MFA_WIZARD_STEPS: ConfigureMfaStepValue[] = [
  ConfigureMfaStep.Configuration,
  ConfigureMfaStep.Fido2,
  ConfigureMfaStep.RecoveryCodes,
  ConfigureMfaStep.Finish,
];

type MfaFactor = {
  method: MfaMethodValue;
  /** Factors sharing a step are configured one after another on it. */
  step: ConfigureMfaStepValue;
  /** Whether the account can hold several of these, so it stays offerable once configured. */
  repeatable: boolean;
};

/** Every factor this client can set up, in selection and wizard order. */
export const MFA_CONFIGURABLE_FACTORS: MfaFactor[] = [
  { method: MfaMethod.Totp, step: ConfigureMfaStep.Configuration, repeatable: false },
  { method: MfaMethod.Email, step: ConfigureMfaStep.Configuration, repeatable: false },
  { method: MfaMethod.Fido2, step: ConfigureMfaStep.Fido2, repeatable: true },
];

export const mfaFactor = (method: MfaMethodValue): MfaFactor | undefined =>
  MFA_CONFIGURABLE_FACTORS.find((factor) => factor.method === method);

export const mfaFactorStep = (
  method: MfaMethodValue,
): ConfigureMfaStepValue | undefined => mfaFactor(method)?.step;

/** Whether the step sets a factor up, as opposed to closing the flow. */
export const isMfaSetupStep = (step: ConfigureMfaStepValue): boolean =>
  MFA_CONFIGURABLE_FACTORS.some((factor) => factor.step === step);

/** The steps the given factors are set up on, in wizard order and without repeats. */
export const mfaStepsOf = (methods: MfaMethodValue[]): ConfigureMfaStepValue[] =>
  MFA_WIZARD_STEPS.filter((step) =>
    methods.some((method) => mfaFactorStep(method) === step),
  );

export const isMfaFactorOfferable = (
  method: MfaMethodValue,
  configuredMethods: MfaMethodValue[],
): boolean => {
  const factor = mfaFactor(method);
  if (!factor) return false;
  return factor.repeatable || !configuredMethods.includes(method);
};

/** Factors that can authorize a session, most preferred first. Core only accepts code factors. */
export const MFA_VERIFICATION_METHODS = [MfaMethod.Totp, MfaMethod.Email] as const;

export type MfaVerificationMethod = (typeof MFA_VERIFICATION_METHODS)[number];

/** Code factors are the only ones a session reports, the rest come from the instance snapshot. */
export const isCodeMfaMethod = (method: MfaMethodValue): boolean =>
  MFA_VERIFICATION_METHODS.some((code) => code === method);
