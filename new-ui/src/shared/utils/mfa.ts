import {
  ConnectionType,
  type LocationInfo,
  LocationMfaMode,
  MfaMethod,
  type MfaMethodValue,
  type MfaStepMethod,
} from '../rust-api/types';

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
  location: Pick<LocationInfo, 'connection_type' | 'location_mfa_mode' | 'mfa_steps'>,
): boolean =>
  location.connection_type !== ConnectionType.Tunnel &&
  (location.mfa_steps.length > 0 ||
    location.location_mfa_mode !== LocationMfaMode.Disabled);

export const mfaStepCount = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
): number =>
  location.connection_type !== ConnectionType.Tunnel ? location.mfa_steps.length : 0;

export const isMfaMethodUsable = (entry: MfaStepMethod): boolean =>
  entry.configured && entry.method !== MfaMethod.Biometric;

/**
 * A step with no usable factor cannot be passed on the desktop. This shouldn't happen
 * because such configuration won't be sent from core.
 * TODO: block connecting only until the user can configure the missing factors in place.
 */
export const hasUnpassableMfaStep = (
  location: Pick<LocationInfo, 'mfa_steps'>,
): boolean => location.mfa_steps.some((step) => !step.methods.some(isMfaMethodUsable));

export const mfaStepsToText = (stepCount: number): string =>
  `${stepCount}-step verification`;
