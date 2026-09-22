import {
  ConnectionType,
  type InstanceInfo,
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
  [MfaMethod.Fido2]: 'Security key',
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

/** Biometric is the mobile client's to drive; the desktop can run the rest. */
export const isDesktopDrivableMethod = (method: MfaMethodValue): boolean =>
  method !== MfaMethod.Biometric;

const isDesktopDrivable = (entry: MfaStepMethod): boolean =>
  isDesktopDrivableMethod(entry.method);

/** Whether the desktop can carry this method for the user as it stands. */
export const isMfaMethodUsable = (
  entry: MfaStepMethod,
  instance?: Pick<InstanceInfo, 'mfa_configured_methods'>,
): boolean => isDesktopDrivable(entry) && isMfaMethodConfigured(entry, instance);

export const usableMfaMethods = (
  step: MfaStep,
  instance?: Pick<InstanceInfo, 'mfa_configured_methods'>,
): MfaStepMethod[] => step.methods.filter((entry) => isMfaMethodUsable(entry, instance));

export const pickableMfaMethods = (step: MfaStep): MfaStepMethod[] => {
  const drivable = step.methods.filter(isDesktopDrivable);
  return drivable.length > 0 ? drivable : step.methods;
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
 * Whether the user has this factor set up. The instance's own report of the
 * account's factors wins when it has one; an instance that predates that API
 * (`mfa_configured_methods === null`) falls back to the flag Core sent with the
 * step, and an unknown instance to the step flag as well.
 */
export const isMfaMethodConfigured = (
  entry: MfaStepMethod,
  instance?: Pick<InstanceInfo, 'mfa_configured_methods'>,
): boolean => {
  const configuredMethods = instance?.mfa_configured_methods;
  return isPresent(configuredMethods)
    ? configuredMethods.includes(entry.method)
    : entry.configured;
};

/**
 * Factors this client can set up on its own, so a step missing only these is one the user can
 * unblock without leaving the app. Email assumes the instance has SMTP; it is never reported.
 */
export const CLIENT_CONFIGURABLE_METHODS = [
  MfaMethod.Totp,
  MfaMethod.Email,
  MfaMethod.Fido2,
] as const;

export type ClientConfigurableMethod = (typeof CLIENT_CONFIGURABLE_METHODS)[number];

export const isClientConfigurableMethod = (method: MfaMethodValue): boolean =>
  CLIENT_CONFIGURABLE_METHODS.some((candidate) => candidate === method);

/** How far the user can get connecting this location with the factors they hold. */
export const ConnectionAbility = {
  /** A whole path through the steps runs on factors already on the account. */
  Available: 'available',
  /** Blocked, but every blocking step offers a factor this client can set up. */
  Configurable: 'configurable',
  /** Blocked on a factor the client cannot set up - the mobile client's, or an
   *  instance too old to configure factors from here. */
  Unavailable: 'unavailable',
} as const;

export type ConnectionAbilityValue =
  (typeof ConnectionAbility)[keyof typeof ConnectionAbility];

/**
 * Single source of truth for whether a location is connectable, and if not,
 * whether configuring factors would fix it. A location with no MFA steps - a
 * bare tunnel, or MFA disabled - is always `Available`.
 */
export const connectionAbilityOf = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
  instance?: Pick<InstanceInfo, 'mfa_configured_methods'>,
): ConnectionAbilityValue => {
  const blockedSteps = mfaStepsOf(location).filter(
    (step) => usableMfaMethods(step, instance).length === 0,
  );
  if (blockedSteps.length === 0) return ConnectionAbility.Available;

  // An instance that never reported its factors cannot configure them from here.
  if (!isPresent(instance?.mfa_configured_methods)) return ConnectionAbility.Unavailable;

  const isFixable = (step: MfaStep): boolean =>
    step.methods.some(
      (entry) => isDesktopDrivable(entry) && isClientConfigurableMethod(entry.method),
    );

  return blockedSteps.every(isFixable)
    ? ConnectionAbility.Configurable
    : ConnectionAbility.Unavailable;
};

export const mfaStepsToText = (stepCount: number): string =>
  `${stepCount}-step verification`;
