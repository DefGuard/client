import { MfaMethod, type MfaMethodValue } from '../../../shared/rust-api/types';
import {
  CLIENT_CONFIGURABLE_METHODS,
  type ClientConfigurableMethod,
} from '../../../shared/utils/mfa';
import {
  ConfigureMfaStep,
  type ConfigureMfaStepValue,
  MFA_VERIFICATION_METHODS,
  MFA_WIZARD_STEPS,
  type MfaFactor,
  type MfaVerificationMethod,
} from './types';

/** Keyed on the shared list, so a new factor fails to compile until mapped here. */
const WIZARD_FACTORS: Record<ClientConfigurableMethod, Omit<MfaFactor, 'method'>> = {
  [MfaMethod.Totp]: { step: ConfigureMfaStep.Configuration, repeatable: false },
  [MfaMethod.Email]: { step: ConfigureMfaStep.Configuration, repeatable: false },
  [MfaMethod.Fido2]: { step: ConfigureMfaStep.Fido2, repeatable: true },
};

/** In selection and wizard order. */
export const MFA_CONFIGURABLE_FACTORS: MfaFactor[] = CLIENT_CONFIGURABLE_METHODS.map(
  (method) => ({ method, ...WIZARD_FACTORS[method] }),
);

export const mfaFactor = (method: MfaMethodValue): MfaFactor | undefined =>
  MFA_CONFIGURABLE_FACTORS.find((factor) => factor.method === method);

export const mfaFactorStep = (
  method: MfaMethodValue,
): ConfigureMfaStepValue | undefined => mfaFactor(method)?.step;

export const isMfaSetupStep = (step: ConfigureMfaStepValue): boolean =>
  MFA_CONFIGURABLE_FACTORS.some((factor) => factor.step === step);

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

/** A session reports only code factors, others come from the instance snapshot. */
export const isCodeMfaMethod = (method: MfaMethodValue): boolean =>
  MFA_VERIFICATION_METHODS.some((code) => code === method);

/** Most preferred first. */
export const verificationMethodsOf = (
  configuredMethods: MfaMethodValue[],
): MfaVerificationMethod[] =>
  MFA_VERIFICATION_METHODS.filter((method) => configuredMethods.includes(method));
