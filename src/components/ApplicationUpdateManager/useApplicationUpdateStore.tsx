import { createWithEqualityFn } from 'zustand/traditional';

export interface ApplicationUpdateStore {
  currentVersion: string | undefined;
  latestVersion: string | undefined;
  releaseDate: string | undefined;
  releaseNotesUrl: string | undefined;
  updateUrl: string | undefined;
  dismissed: boolean;
  setValues: (values: Partial<ApplicationUpdateStore>) => void;
}

const defaultState = {
  currentVersion: undefined,
  latestVersion: undefined,
  releaseDate: undefined,
  releaseNotesUrl: undefined,
  updateUrl: undefined,
  dismissed: false,
} as ApplicationUpdateStore;

export const useApplicationUpdateStore = createWithEqualityFn<ApplicationUpdateStore>(
  (set) => ({
    ...defaultState,
    setValues: (values: Partial<ApplicationUpdateStore>) => set({ ...values }),
  }),
  Object.is,
);
