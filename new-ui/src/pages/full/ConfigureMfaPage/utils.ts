import {
  type MfaConfigStartResult,
  MfaMethod,
  type MfaMethodValue,
} from '../../../shared/rust-api/types';
import {
  CLIENT_CONFIGURABLE_METHODS,
  type ClientConfigurableMethod,
} from '../../../shared/utils/mfa';
import {
  ConfigureMfaStep,
  type ConfigureMfaStepValue,
  MFA_WIZARD_STEPS,
  type MfaFactor,
  type MfaVerificationMethod,
  mfaVerificationMethods,
} from './types';

/** Keyed on the shared list, so a new factor fails to compile until mapped here. */
const WIZARD_FACTORS: Record<ClientConfigurableMethod, Omit<MfaFactor, 'method'>> = {
  [MfaMethod.Totp]: { step: ConfigureMfaStep.Configuration, repeatable: false },
  [MfaMethod.Email]: { step: ConfigureMfaStep.Configuration, repeatable: false },
  [MfaMethod.Fido2]: { step: ConfigureMfaStep.Fido2, repeatable: true },
};

/** In selection and wizard order. */
const configurableFactors: MfaFactor[] = CLIENT_CONFIGURABLE_METHODS.map((method) => ({
  method,
  ...WIZARD_FACTORS[method],
}));

export const mfaFactor = (method: MfaMethodValue): MfaFactor | undefined =>
  configurableFactors.find((factor) => factor.method === method);

export const mfaFactorStep = (
  method: MfaMethodValue,
): ConfigureMfaStepValue | undefined => mfaFactor(method)?.step;

export const isMfaSetupStep = (step: ConfigureMfaStepValue): boolean =>
  configurableFactors.some((factor) => factor.step === step);

export const mfaStepsOf = (methods: MfaMethodValue[]): ConfigureMfaStepValue[] =>
  MFA_WIZARD_STEPS.filter((step) =>
    methods.some((method) => mfaFactorStep(method) === step),
  );

export const isMfaFactorOfferable = (
  method: MfaMethodValue,
  configuredMethods: MfaMethodValue[],
  setupMethods: MfaMethodValue[],
): boolean => {
  const factor = mfaFactor(method);
  if (!factor || !setupMethods.includes(method)) return false;
  return factor.repeatable || !configuredMethods.includes(method);
};

const codeMfaMethods: MfaMethodValue[] = [MfaMethod.Totp, MfaMethod.Email];

/** every Core reports code factors in a session but older ones leave out FIDO2,
 *  so the instance snapshot stays the source for the rest */
export const isCodeMfaMethod = (method: MfaMethodValue): boolean =>
  codeMfaMethods.includes(method);

/** ordered by preference, not by the input */
export const verificationMethodsOf = (
  availableMethods: MfaMethodValue[],
): MfaVerificationMethod[] =>
  mfaVerificationMethods.filter((method) => availableMethods.includes(method));

/** The fallback mails a code to the address on file, so email is the only way in. */
export const sessionVerificationMethodsOf = (
  response: Pick<MfaConfigStartResult, 'email_fallback' | 'available_methods'>,
): MfaVerificationMethod[] =>
  verificationMethodsOf(
    response.email_fallback ? [MfaMethod.Email] : response.available_methods,
  );
