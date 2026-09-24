import { MfaMethod, type MfaMethodValue } from '../../../rust-api/types';

export const LocationCardViews = {
  Default: 'default',
  MfaTotp: 'mfa-totp',
  MfaEmail: 'mfa-email',
  MfaOidc: 'mfa-oidc',
  MfaMobile: 'mfa-mobile',
  MfaFido2: 'mfa-fido2',
  MfaSettings: 'mfa-settings',
  Connecting: 'connecting',
  Connected: 'connected',
  PostureCheckFail: 'posture-check-fail',
  ConnectionError: 'connection-error',
} as const;

export type LocationCardViewsValue =
  (typeof LocationCardViews)[keyof typeof LocationCardViews];

export const mfaMethodToLocationCardView = (
  method: MfaMethodValue,
): LocationCardViewsValue => {
  switch (method) {
    case MfaMethod.Email:
      return LocationCardViews.MfaEmail;
    case MfaMethod.Oidc:
      return LocationCardViews.MfaOidc;
    case MfaMethod.MobileApprove:
      return LocationCardViews.MfaMobile;
    case MfaMethod.Fido2:
      return LocationCardViews.MfaFido2;
    default:
      return LocationCardViews.MfaTotp;
  }
};
