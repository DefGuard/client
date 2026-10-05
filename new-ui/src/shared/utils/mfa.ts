import { OpenIdProvider } from '../consts';
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
  [MfaMethod.MobileApprove]: 'Mobile App Approval',
  [MfaMethod.Oidc]: 'OpenID',
  [MfaMethod.Totp]: 'Authenticator app',
  [MfaMethod.Biometric]: 'Mobile Biometric Authentication',
  [MfaMethod.Fido2]: 'Security key',
};

type OpenIdInstance = Pick<InstanceInfo, 'openid_display_name' | 'openid_provider_kind'>;

export const openIdProviderName = (instance?: OpenIdInstance): string =>
  instance?.openid_display_name || mfaMethodLabels[MfaMethod.Oidc];

export const mfaToText = (factor: MfaMethodValue, instance?: OpenIdInstance): string => {
  if (factor === MfaMethod.Oidc) {
    return openIdProviderName(instance);
  }
  return mfaMethodLabels[factor];
};

export const findOpenIdProvider = (instance?: OpenIdInstance) =>
  Object.values(OpenIdProvider).find(
    (provider) => provider === instance?.openid_provider_kind,
  );

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

export type MfaAvailabilityInstance = Pick<
  InstanceInfo,
  'smtp_configured' | 'openid_available'
>;

/** Whether the instance can run this factor at all. An unreported flag counts as available. */
export const isMfaMethodAvailable = (
  method: MfaMethodValue,
  instance?: MfaAvailabilityInstance | null,
): boolean => {
  switch (method) {
    case MfaMethod.Email:
      return instance?.smtp_configured !== false;
    case MfaMethod.Oidc:
      return instance?.openid_available !== false;
    default:
      return true;
  }
};

type MfaUsabilityInstance = Pick<InstanceInfo, 'mfa_configured_methods'> &
  MfaAvailabilityInstance;

/** Whether the desktop can carry this method for the user as it stands. */
export const isMfaMethodUsable = (
  entry: MfaStepMethod,
  instance?: MfaUsabilityInstance,
): boolean =>
  isDesktopDrivable(entry) &&
  isMfaMethodConfigured(entry, instance) &&
  isMfaMethodAvailable(entry.method, instance);

export const usableMfaMethods = (
  step: MfaStep,
  instance?: MfaUsabilityInstance,
): MfaStepMethod[] => step.methods.filter((entry) => isMfaMethodUsable(entry, instance));

export const pickableMfaMethods = (step: MfaStep): MfaStepMethod[] => {
  const drivable = step.methods.filter(isDesktopDrivable);
  return drivable.length > 0 ? drivable : step.methods;
};

/** gates the MFA settings view, which has nothing to pick otherwise */
export const hasMfaMethodChoice = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
  instance?: MfaAvailabilityInstance | null,
): boolean =>
  mfaStepsOf(location).some(
    (step) =>
      pickableMfaMethods(step).filter((entry) =>
        isMfaMethodAvailable(entry.method, instance),
      ).length > 1,
  );

export const resolveMfaStepPlan = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps' | 'mfa_step_plan'>,
  oneOffPlan: MfaMethodValue[] = [],
  instance?: MfaUsabilityInstance,
): MfaMethodValue[] =>
  mfaStepsOf(location).map((step, index) => {
    const usableMethods = usableMfaMethods(step, instance);
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
 * unblock without leaving the app.
 */
export const CLIENT_CONFIGURABLE_METHODS = [
  MfaMethod.Totp,
  MfaMethod.Email,
  MfaMethod.Fido2,
] as const;

export type ClientConfigurableMethod = (typeof CLIENT_CONFIGURABLE_METHODS)[number];

export const isClientConfigurableMethod = (
  method: MfaMethodValue,
): method is ClientConfigurableMethod =>
  CLIENT_CONFIGURABLE_METHODS.some((candidate) => candidate === method);

type MfaCapabilitiesInstance = Pick<InstanceInfo, 'mfa_capabilities'> &
  MfaAvailabilityInstance;

/** in client order, which the picker and wizard rely on */
export const setupMethodsOf = (
  instance?: MfaCapabilitiesInstance | null,
): ClientConfigurableMethod[] => {
  const coreSetupMethods = instance?.mfa_capabilities?.setup_methods ?? [];
  return CLIENT_CONFIGURABLE_METHODS.filter(
    (method) =>
      coreSetupMethods.includes(method) && isMfaMethodAvailable(method, instance),
  );
};

export const canSetUpMfaMethod = (
  method: MfaMethodValue,
  instance?: MfaCapabilitiesInstance | null,
): method is ClientConfigurableMethod =>
  setupMethodsOf(instance).some((candidate) => candidate === method);

/** What this client can authorize an MFA configuration session with. Mirrors
 *  AUTHORIZING_METHODS in core mfa_config.rs. */
const clientAuthorizingMethods: MfaMethodValue[] = [
  MfaMethod.Totp,
  MfaMethod.Email,
  MfaMethod.Fido2,
  MfaMethod.Oidc,
];

export type MfaConfigInstance = Pick<InstanceInfo, 'mfa_configured_methods'> &
  MfaCapabilitiesInstance;

/**
 * Whether a configuration session could be authorized, by a factor the account holds and the
 * instance accepts, or by the email fallback Core offers an account with none of those.
 */
export const canAuthorizeMfaConfig = (instance?: MfaConfigInstance | null): boolean => {
  const authorizeMethods = instance?.mfa_capabilities?.authorize_methods ?? [];
  const configuredMethods = instance?.mfa_configured_methods ?? [];
  const holdsAuthorizer = clientAuthorizingMethods.some(
    (method) =>
      authorizeMethods.includes(method) &&
      configuredMethods.includes(method) &&
      isMfaMethodAvailable(method, instance),
  );
  return holdsAuthorizer || isMfaMethodAvailable(MfaMethod.Email, instance);
};

/** How far the user can get connecting this location with the factors they hold. */
export const ConnectionAbility = {
  /** A whole path through the steps runs on factors already on the account. */
  Available: 'available',
  /** Blocked, but every blocking step offers a factor this client can set up there. */
  Configurable: 'configurable',
  /** Blocked on a factor that cannot be set up here, or with no way to authorize the setup. */
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
  instance?: MfaConfigInstance,
): ConnectionAbilityValue => {
  const blockedSteps = mfaStepsOf(location).filter(
    (step) => usableMfaMethods(step, instance).length === 0,
  );
  if (blockedSteps.length === 0) return ConnectionAbility.Available;

  const isFixable = (step: MfaStep): boolean =>
    step.methods.some((entry) => canSetUpMfaMethod(entry.method, instance));

  return blockedSteps.every(isFixable) && canAuthorizeMfaConfig(instance)
    ? ConnectionAbility.Configurable
    : ConnectionAbility.Unavailable;
};

export const mfaStepsToText = (stepCount: number): string =>
  `${stepCount}-step verification`;
