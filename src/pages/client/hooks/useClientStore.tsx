import { isUndefined } from 'lodash-es';
import { createWithEqualityFn } from 'zustand/traditional';

import { clientApi } from '../clientAPI/clientApi';
import { ClientView, DefguardInstance } from '../types';

const { getInstances } = clientApi;

// eslint-disable-next-line
const defaultValues: StoreValues = {
  instances: [],
  selectedInstance: undefined,
  statsFilter: 1,
  selectedView: ClientView.GRID,
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
    updateInstances: async () => {
      const res = await getInstances();
      let selected = get().selectedInstance;
      // check if currently selected instances is in updated instances
      if (!isUndefined(selected) && res.length) {
        if (!res.map((i) => i.id).includes(selected)) {
          selected = res[0].id;
        }
      }
      if (isUndefined(selected) && res.length) {
        selected = res[0].id;
      }
      set({ instances: res, selectedInstance: selected });
    },
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  instances: DefguardInstance[];
  selectedView: ClientView;
  statsFilter: number;
  selectedInstance?: DefguardInstance['id'];
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  setInstances: (instances: DefguardInstance[]) => void;
  updateInstances: () => Promise<void>;
};
