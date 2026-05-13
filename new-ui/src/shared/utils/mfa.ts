import type { MfaMethodValue } from '../rust-api/types';

export const mfaToText = (factor: MfaMethodValue): string => {
  switch (factor) {
    case 'Email':
      return 'Email';
    case 'MobileApprove':
      return 'Mobile Client';
    case 'Oidc':
      return 'OpenID';
    case 'Totp':
      return 'Authenticator app';
  }
};
