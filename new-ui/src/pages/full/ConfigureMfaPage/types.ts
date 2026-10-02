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

export type MfaFactor = {
  method: MfaMethodValue;
  /** Factors sharing a step are configured one after another on it. */
  step: ConfigureMfaStepValue;
  /** Whether the account can hold several of these, so it stays offerable once configured. */
  repeatable: boolean;
};

/** most preferred first */
export const mfaVerificationMethods = [
  MfaMethod.Totp,
  MfaMethod.Email,
  MfaMethod.Fido2,
  MfaMethod.Oidc,
] as const;

export type MfaVerificationMethod = (typeof mfaVerificationMethods)[number];
