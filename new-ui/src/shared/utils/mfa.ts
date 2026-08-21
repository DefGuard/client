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
};

export const mfaToText = (factor: MfaMethodValue): string => mfaMethodLabels[factor];

export const mfaMethodApiValues: Record<MfaMethodValue, string> = {
  [MfaMethod.Email]: 'Email',
  [MfaMethod.MobileApprove]: 'MobileApprove',
  [MfaMethod.Oidc]: 'Oidc',
  [MfaMethod.Totp]: 'Totp',
  [MfaMethod.Biometric]: 'Biometric',
};

export const mfaToApi = (factor: MfaMethodValue): string => mfaMethodApiValues[factor];

/**
 * Whether connecting this location should trigger the MFA flow: only for
 * server-managed locations (never bare tunnels) that have MFA enabled.
 */
export const shouldStartMfa = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): boolean => mfaStepCount(location) > 0;

export const mfaStepCount = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): number =>
  location.connection_type !== ConnectionType.Tunnel ? location.mfa_steps.length : 0;

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
  location: Pick<LocationInfo, 'mfa_steps' | 'mfa_step_plan'>,
  oneOffPlan: MfaMethodValue[] = [],
): MfaMethodValue[] =>
  location.mfa_steps.map((step, index) => {
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
  location: Pick<LocationInfo, 'mfa_steps'>,
): boolean => location.mfa_steps.some((step) => usableMfaMethods(step).length === 0);

export const mfaStepsToText = (stepCount: number): string =>
  `${stepCount}-step verification`;
