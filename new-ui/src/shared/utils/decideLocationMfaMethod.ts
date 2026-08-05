import { type LocationInfo, MfaMethod, type MfaMethodValue } from '../rust-api/types';

export const decideLocationMfaMethod = (
  location: LocationInfo,
  currentMethod: MfaMethodValue | null | undefined,
): MfaMethodValue | null => {
  switch (location.location_mfa_mode) {
    case 'disabled':
      return location.user_mfa_preference?.[0] ?? null;
    case 'external':
      return MfaMethod.Oidc;
    case 'internal':
      if (currentMethod === MfaMethod.Oidc || !currentMethod)
        return location.user_mfa_preference?.[0] ?? null;
      return currentMethod;
  }
};
