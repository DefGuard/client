import { MfaMethod, type MfaMethodValue } from '../../../../../../shared/rust-api/types';

export const ConnectModalView = {
  MfaTotp: 'mfa-totp',
  MfaEmail: 'mfa-email',
  MfaOidc: 'mfa-oidc',
  MfaMobile: 'mfa-mobile',
  MfaSettings: 'mfa-settings',
  PostureCheckFail: 'posture-check-fail',
} as const;

/** Map an MFA method to the ConnectModal view that collects it. */
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
    default:
      return ConnectModalView.MfaTotp;
  }
};

export const ConnectModalTitle: Record<ConnectModalViewValue, string> = {
  [ConnectModalView.MfaTotp]: 'Two-factor authentication',
  [ConnectModalView.MfaEmail]: 'Two-factor authentication',
  [ConnectModalView.MfaOidc]: 'Two-factor authentication',
  [ConnectModalView.MfaMobile]: 'Two-factor authentication',
  [ConnectModalView.MfaSettings]: 'Change MFA Method',
  [ConnectModalView.PostureCheckFail]: 'Access denied',
} as const;

export type ConnectModalViewValue =
  (typeof ConnectModalView)[keyof typeof ConnectModalView];
