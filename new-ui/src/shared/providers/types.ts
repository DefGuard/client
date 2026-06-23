import type { MfaMethodValue, OverviewViewSelection } from '../rust-api/types';

export type { OverviewViewSelection };

export type SharedSessionStorage = {
  viewSelection: OverviewViewSelection | null;
  connectionMfaMethod: Record<string, MfaMethodValue | undefined>;
};
