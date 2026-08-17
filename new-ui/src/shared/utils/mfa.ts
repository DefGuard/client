import {
  ConnectionType,
  type LocationInfo,
  LocationMfaMode,
  MfaMethod,
  type MfaMethodValue,
} from '../rust-api/types';

const mfaMethodLabels: Record<MfaMethodValue, string> = {
  [MfaMethod.Email]: 'Email',
  [MfaMethod.MobileApprove]: 'Mobile Client',
  [MfaMethod.Oidc]: 'OpenID',
  [MfaMethod.Totp]: 'Authenticator app',
  [MfaMethod.Biometric]: 'Biometric',
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
 * Whether connecting this location should trigger the MFA flow: only for
 * server-managed locations (never bare tunnels) that have MFA enabled.
 */
export const shouldStartMfa = (
  location: Pick<LocationInfo, 'connection_type' | 'location_mfa_mode'>,
): boolean =>
  location.connection_type !== ConnectionType.Tunnel &&
  location.location_mfa_mode !== LocationMfaMode.Disabled;
