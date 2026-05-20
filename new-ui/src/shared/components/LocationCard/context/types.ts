export const LocationCardViews = {
  Default: 'default',
  MfaTotp: 'mfa-totp',
  MfaEmail: 'mfa-email',
  MfaOidc: 'mfa-oidc',
  MfaMobile: 'mfa-mobile',
  MfaSettings: 'mfa-settings',
  Connecting: 'connecting',
  Connected: 'connected',
  PostureCheckFail: 'posture-check-fail',
} as const;

export type LocationCardViewsValue =
  (typeof LocationCardViews)[keyof typeof LocationCardViews];
