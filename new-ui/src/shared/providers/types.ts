import type { MfaMethodValue, OverviewViewSelection } from '../rust-api/types';

export type { OverviewViewSelection };

export type SharedSessionStorage = {
  viewSelection: OverviewViewSelection | null;
  locationMfaPreference: Record<string, MfaMethodValue | undefined>;
};
