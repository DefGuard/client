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

/**
 * MFA steps the connect flow runs on, exactly as Core configured them - never
 * for bare tunnels. Always read steps through this instead of
 * `location.mfa_steps`.
 *
 * FIDO2 is listed here like any other method, including its `configured` flag:
 * the key signs a challenge for a credential Core registered for this user, so
 * a key that was never registered cannot pass the step, and offering it anyway
 * only earns a rejected plan from Edge.
 */
export const mfaStepsOf = (
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
 * A step the desktop cannot drive at all blocks connecting: every method in it
 * needs the mobile client, so there is nothing the user could do here.
 *
 * A method Core reports as not yet configured does NOT block. Whether a factor
 * can actually be used is Core's call, and Edge says so with a message the user
 * can act on - "set it up first, or pick a different one" - which beats a mute
 * disabled button that explains nothing.
 */
export const hasUnpassableMfaStep = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): boolean =>
  mfaStepsOf(location).some(
    (step) => !step.methods.some((entry) => entry.method !== MfaMethod.Biometric),
  );

export const mfaStepsToText = (stepCount: number): string =>
  `${stepCount}-step verification`;
