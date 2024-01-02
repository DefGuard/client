import { isUndefined } from 'lodash-es';
import { createWithEqualityFn } from 'zustand/traditional';

import { clientApi } from '../clientAPI/clientApi';
import { Settings } from '../clientAPI/types';
import {
  ClientView,
  CommonWireguardFields,
  DefguardInstance,
  SelectedInstance,
  WireguardInstanceType,
} from '../types';

const { getInstances, updateSettings } = clientApi;

// eslint-disable-next-line
const defaultValues: StoreValues = {
  instances: [],
  tunnels: [],
  selectedInstance: undefined,
  statsFilter: 1,
  selectedView: ClientView.GRID,
  settings: {
    log_level: 'error',
    theme: 'light',
    tray_icon_theme: 'color',
  },
};

export const useClientStore = createWithEqualityFn<Store>(
  (set, get) => ({
    ...defaultValues,
    setState: (values) => set({ ...values }),
    setInstances: (values) => {
      if (isUndefined(get().selectedInstance)) {
        return set({
          instances: values,
          selectedInstance:
            { id: values[0]?.id, type: WireguardInstanceType.DEFGUARD_INSTANCE } ??
            undefined,
        });
      }
      return set({ instances: values });
    },
    updateInstances: async () => {
      const res = await getInstances();
      let selected = get().selectedInstance;
      // check if currently selected instances is in updated instances
      if (!isUndefined(selected) && res.length && selected.id) {
        if (!res.map((i) => i.id).includes(selected.id)) {
          selected = { id: res[0].id, type: WireguardInstanceType.DEFGUARD_INSTANCE };
        }
      }
      if (isUndefined(selected) && res.length) {
        selected = { id: res[0].id, type: WireguardInstanceType.DEFGUARD_INSTANCE };
      }
      set({ instances: res, selectedInstance: selected });
    },
    updateSettings: async (data) => {
      const res = await updateSettings(data);
      set({ settings: res });
    },
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  instances: DefguardInstance[];
  tunnels: CommonWireguardFields[];
  selectedView: ClientView;
  statsFilter: number;
  settings: Settings;
  selectedInstance?: SelectedInstance;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  setInstances: (instances: DefguardInstance[]) => void;
  updateInstances: () => Promise<void>;
  updateSettings: (data: Partial<Settings>) => Promise<void>;
};
