import type { MfaMethod } from '../rust-api/types';

export const mfaToText = (factor: MfaMethod): string => {
  switch (factor) {
    case 'email':
      return 'Email';
    case 'mobileapprove':
      return 'Mobile Client';
    case 'oidc':
      return 'OpenID';
    case 'totp':
      return 'Authenticator app';
    case 'biometric':
      return 'Biometric';
  }
};
