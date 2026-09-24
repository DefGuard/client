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

export const motionTransitionStandard = {
  type: 'tween',
  ease: 'easeOut',
  duration: 0.16,
} as const;

export const WindowId = {
  FullView: 'full-view',
  CompactView: 'compact-view',
} as const;
