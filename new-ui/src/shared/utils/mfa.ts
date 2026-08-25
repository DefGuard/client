import {
  ConnectionType,
  type LocationInfo,
  MfaMethod,
  type MfaMethodValue,
  type MfaStep,
  type MfaStepMethod,
} from '../rust-api/types';
import { isPresent } from './isPresent';

const mfaMethodLabels: Record<MfaMethodValue, string> = {
  [MfaMethod.Email]: 'Email',
  [MfaMethod.MobileApprove]: 'Mobile Client',
  [MfaMethod.Oidc]: 'OpenID',
  [MfaMethod.Totp]: 'Authenticator app',
  [MfaMethod.Biometric]: 'Biometrics',
  [MfaMethod.Fido2]: 'Security key (FIDO2)',
};

export const mfaToText = (factor: MfaMethodValue): string => mfaMethodLabels[factor];

export const mfaMethodApiValues: Record<MfaMethodValue, string> = {
  [MfaMethod.Email]: 'Email',
  [MfaMethod.MobileApprove]: 'MobileApprove',
  [MfaMethod.Oidc]: 'Oidc',
  [MfaMethod.Totp]: 'Totp',
  [MfaMethod.Biometric]: 'Biometric',
  [MfaMethod.Fido2]: 'Fido2',
};

export const mfaToApi = (factor: MfaMethodValue): string => mfaMethodApiValues[factor];

const FIDO2_STEP_METHOD: MfaStepMethod = {
  method: MfaMethod.Fido2,
  configured: true,
};

/**
 * The client verifies FIDO2 itself, against the key the user has in hand, so it
 * needs no prior server-side registration: wherever a step offers FIDO2 it
 * counts as configured, even when the server marks it otherwise.
 *
 * The protocol cannot carry FIDO2 yet, so a step that does not mention it gets
 * the entry synthesized, under two limits:
 *
 * - single-step locations only, because a locally verified step inside a
 *   multi-step plan would desynchronize the server-side step sequence;
 * - never on a step the identity provider owns outright (OIDC only), where the
 *   server accepts no other factor.
 *
 * Drop the synthesis - not the normalization - once FIDO2 reaches `mfa_steps`
 * on its own.
 */
const withClientVerifiedMethods = (steps: MfaStep[]): MfaStep[] => {
  const normalized = steps.map((step) => ({
    methods: step.methods.map((entry) =>
      entry.method === MfaMethod.Fido2 ? FIDO2_STEP_METHOD : entry,
    ),
  }));

  const onlyStep = normalized.length === 1 ? normalized[0] : undefined;
  if (!isPresent(onlyStep)) return normalized;

  const offersFido2 = onlyStep.methods.some((entry) => entry.method === MfaMethod.Fido2);
  const isExternallyOwned = onlyStep.methods.every(
    (entry) => entry.method === MfaMethod.Oidc,
  );
  if (offersFido2 || isExternallyOwned) return normalized;

  return [{ methods: [...onlyStep.methods, FIDO2_STEP_METHOD] }];
};

/**
 * MFA steps the connect flow runs on: never for bare tunnels, and with the
 * client-verified factors folded into the server-provided ones. Always read
 * steps through this instead of `location.mfa_steps`.
 */
export const mfaStepsOf = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): MfaStep[] => withClientVerifiedMethods(serverMfaSteps(location));

/** The steps exactly as the server configured them. */
const serverMfaSteps = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): MfaStep[] =>
  location.connection_type === ConnectionType.Tunnel ? [] : location.mfa_steps;

/**
 * Whether connecting this location should trigger the MFA flow: only for
 * server-managed locations (never bare tunnels) that have MFA enabled.
 */
export const shouldStartMfa = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): boolean => mfaStepCount(location) > 0;

export const mfaStepCount = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): number => mfaStepsOf(location).length;

export const usableMfaMethods = (step: MfaStep): MfaStepMethod[] =>
  step.methods.filter(
    (entry) => entry.configured && entry.method !== MfaMethod.Biometric,
  );

export const pickableMfaMethods = (step: MfaStep): MfaStepMethod[] => {
  const withoutBiometric = step.methods.filter(
    (entry) => entry.method !== MfaMethod.Biometric,
  );
  return withoutBiometric.length > 0 ? withoutBiometric : step.methods;
};

export const resolveMfaStepPlan = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps' | 'mfa_step_plan'>,
  oneOffPlan: MfaMethodValue[] = [],
): MfaMethodValue[] =>
  mfaStepsOf(location).map((step, index) => {
    const usableMethods = usableMfaMethods(step);
    const isUsable = (method: MfaMethodValue) =>
      usableMethods.some((entry) => entry.method === method);

    const oneOffChoice = oneOffPlan[index];
    if (isPresent(oneOffChoice) && isUsable(oneOffChoice)) return oneOffChoice;

    const savedChoice = location.mfa_step_plan[index];
    if (isPresent(savedChoice) && isUsable(savedChoice)) return savedChoice;

    return (usableMethods[0] ?? step.methods[0]).method;
  });

/**
 * A step with no usable factor cannot be passed on the desktop. This shouldn't happen
 * because such configuration won't be sent from core.
 * TODO: block connecting only until the user can configure the missing factors in place.
 */
export const hasUnpassableMfaStep = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): boolean => mfaStepsOf(location).some((step) => usableMfaMethods(step).length === 0);

export const mfaStepsToText = (stepCount: number): string =>
  `${stepCount}-step verification`;
