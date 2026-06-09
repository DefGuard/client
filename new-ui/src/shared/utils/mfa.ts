import type { MfaMethodValue } from '../rust-api/types';

export const mfaToNumber = (method: MfaMethodValue): number => {
  switch (method) {
    case 'totp':
      return 0;
    case 'email':
      return 1;
    case 'oidc':
      return 2;
    case 'biometric':
      return 3;
    case 'mobileapprove':
      return 4;
  }
};

export const mfaToText = (factor: MfaMethodValue): string => {
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
