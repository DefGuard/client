import { MfaMethod, type MfaMethodValue } from '../../../../../../shared/rust-api/types';

export const ConnectModalView = {
  MfaTotp: 'mfa-totp',
  MfaEmail: 'mfa-email',
  MfaOidc: 'mfa-oidc',
  MfaMobile: 'mfa-mobile',
  MfaFido2: 'mfa-fido2',
  MfaSettings: 'mfa-settings',
  PostureCheckFail: 'posture-check-fail',
  ConnectionError: 'connection-error',
} as const;

export const mfaMethodToConnectModalView = (
  method: MfaMethodValue,
): ConnectModalViewValue => {
  switch (method) {
    case MfaMethod.Email:
      return ConnectModalView.MfaEmail;
    case MfaMethod.Oidc:
      return ConnectModalView.MfaOidc;
    case MfaMethod.MobileApprove:
      return ConnectModalView.MfaMobile;
    case MfaMethod.Fido2:
      return ConnectModalView.MfaFido2;
    default:
      return ConnectModalView.MfaTotp;
  }
};

export const ConnectModalTitle: Record<ConnectModalViewValue, string> = {
  [ConnectModalView.MfaTotp]: 'Two-factor authentication',
  [ConnectModalView.MfaEmail]: 'Two-factor authentication',
  [ConnectModalView.MfaOidc]: 'Two-factor authentication',
  [ConnectModalView.MfaMobile]: 'Two-factor authentication',
  [ConnectModalView.MfaFido2]: 'Two-factor authentication',
  [ConnectModalView.MfaSettings]: 'Change MFA Method',
  [ConnectModalView.PostureCheckFail]: 'Access denied',
  [ConnectModalView.ConnectionError]: 'Connection error',
} as const;

export type ConnectModalViewValue =
  (typeof ConnectModalView)[keyof typeof ConnectModalView];
