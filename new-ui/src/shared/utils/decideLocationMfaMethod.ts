import { type LocationInfo, MfaMethod, type MfaMethodValue } from '../rust-api/types';

export const decideLocationMfaMethod = (
  location: LocationInfo,
  currentMethod: MfaMethodValue | null | undefined,
): MfaMethodValue | null => {
  switch (location.location_mfa_mode) {
    case 'disabled':
      return location.mfa_method ?? null;
    case 'external':
      return MfaMethod.Oidc;
    case 'internal':
      if (currentMethod === MfaMethod.Oidc || !currentMethod)
        return location.mfa_method ?? null;
      return currentMethod;
  }
};
