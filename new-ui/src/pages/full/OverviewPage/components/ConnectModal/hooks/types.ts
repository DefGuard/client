export const ConnectModalView = {
  MfaTotp: 'mfa-totp',
  MfaEmail: 'mfa-email',
  MfaOidc: 'mfa-oidc',
  MfaMobile: 'mfa-mobile',
  MfaSettings: 'mfa-settings',
  PostureCheckFail: 'posture-check-fail',
  ConnectionError: 'connection-error',
} as const;

export const ConnectModalTitle: Record<ConnectModalViewValue, string> = {
  [ConnectModalView.MfaTotp]: 'Two-factor authentication',
  [ConnectModalView.MfaEmail]: 'Two-factor authentication',
  [ConnectModalView.MfaOidc]: 'Two-factor authentication',
  [ConnectModalView.MfaMobile]: 'Two-factor authentication',
  [ConnectModalView.MfaSettings]: 'Change MFA Method',
  [ConnectModalView.PostureCheckFail]: 'Access denied',
  [ConnectModalView.ConnectionError]: 'Connection error',
} as const;

export type ConnectModalViewValue =
  (typeof ConnectModalView)[keyof typeof ConnectModalView];
