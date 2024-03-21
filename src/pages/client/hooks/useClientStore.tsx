import { createWithEqualityFn } from 'zustand/traditional';

import { clientApi } from '../clientAPI/clientApi';
import { Settings } from '../clientAPI/types';
import {
  ClientView,
  CommonWireguardFields,
  DefguardInstance,
  SelectedInstance,
} from '../types';

const { updateSettings } = clientApi;

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
  (set) => ({
    ...defaultValues,
    setState: (values) => set({ ...values }),
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
  updateSettings: (data: Partial<Settings>) => Promise<void>;
};
