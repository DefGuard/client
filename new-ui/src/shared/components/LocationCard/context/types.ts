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
