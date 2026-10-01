import type { IconKindValue } from './components/Icon';
import { MfaMethod, type MfaMethodValue } from './rust-api/types';

export const mfaMethodIcon: Record<MfaMethodValue, IconKindValue> = {
  [MfaMethod.Email]: 'mail',
  [MfaMethod.MobileApprove]: 'qr',
  [MfaMethod.Oidc]: 'token',
  [MfaMethod.Totp]: 'lock-closed',
  [MfaMethod.Biometric]: 'biometric',
  [MfaMethod.Fido2]: 'software-key',
};

export const OpenIdProvider = {
  Microsoft: 'microsoft',
  Google: 'google',
  Okta: 'okta',
  JumpCloud: 'jumpcloud',
} as const;

export type OpenIdProviderValue = (typeof OpenIdProvider)[keyof typeof OpenIdProvider];

export const openIdProviderIcon: Record<OpenIdProviderValue, IconKindValue> = {
  [OpenIdProvider.Microsoft]: 'microsoft-white',
  [OpenIdProvider.Google]: 'google-white',
  [OpenIdProvider.Okta]: 'okta-white',
  [OpenIdProvider.JumpCloud]: 'jump-cloud-white',
};

export const openIdProviderColorIcon: Record<OpenIdProviderValue, IconKindValue> = {
  [OpenIdProvider.Microsoft]: 'microsoft',
  [OpenIdProvider.Google]: 'google',
  [OpenIdProvider.Okta]: 'okta',
  [OpenIdProvider.JumpCloud]: 'jump-cloud',
};

export const motionTransitionStandard = {
  type: 'tween',
  ease: 'easeOut',
  duration: 0.16,
} as const;

export const WindowId = {
  FullView: 'full-view',
  CompactView: 'compact-view',
} as const;
