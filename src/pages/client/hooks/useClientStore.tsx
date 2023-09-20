import { isUndefined } from 'lodash-es';
import { createWithEqualityFn } from 'zustand/traditional';

import { DefguardInstance } from '../types';

// eslint-disable-next-line
const defaultValues: StoreValues = {
  instances: [],
  selectedInstance: undefined,
};

export const useClientStore = createWithEqualityFn<Store>(
  (set, get) => ({
    ...defaultValues,
    setState: (values) => set({ ...values }),
    setInstances: (values) => {
      if (isUndefined(get().selectedInstance)) {
        return set({ instances: values, selectedInstance: values[0]?.id ?? undefined });
      }
      return set({ instances: values });
    },
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  instances: DefguardInstance[];
  selectedInstance?: DefguardInstance['id'];
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  setInstances: (instances: DefguardInstance[]) => void;
};
