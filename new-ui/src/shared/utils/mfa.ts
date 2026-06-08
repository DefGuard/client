import type { EnrollmentMfaMethod, MfaMethodValue } from '../rust-api/types';

export const mfaMethodToEnrollment = (method: number): EnrollmentMfaMethod => {
  switch (method) {
    case 0:
      return 'Totp';
    case 1:
      return 'Email';
    case 2:
      return 'Oidc';
    case 3:
      return 'Biometric';
    case 4:
      return 'MobileApprove';
    default:
      throw new Error(`Unknown MfaMethod value: ${method}`);
  }
};

export const enrollmentToMfaMethod = (method: EnrollmentMfaMethod): number => {
  switch (method) {
    case 'Totp':
      return 0;
    case 'Email':
      return 1;
    case 'Oidc':
      return 2;
    case 'Biometric':
      return 3;
    case 'MobileApprove':
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
