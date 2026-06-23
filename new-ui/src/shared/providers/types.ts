import type { OverviewViewSelection } from '../rust-api/types';

export type { OverviewViewSelection };

export type SharedSessionStorage = {
  viewSelection: OverviewViewSelection | null;
};
