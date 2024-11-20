import { isUndefined, pickBy } from 'lodash-es';
import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { clientApi } from '../clientAPI/clientApi';
import { AppConfig, ClientView } from '../clientAPI/types';
import {
  CommonWireguardFields,
  DefguardInstance,
  SelectedInstance,
  WireguardInstanceType,
} from '../types';

const { getInstances, setAppConfig } = clientApi;

// eslint-disable-next-line
const defaultValues: StoreValues = {
  instances: [],
  tunnels: [],
  selectedInstance: undefined,
  selectedLocation: undefined,
  statsFilter: 1,
  listChecked: false,
  selectedView: 'grid',
  // application config stored in app data json file, ONLY interact with it via store methods.
  appConfig: {
    log_level: 'INFO',
    theme: 'light',
    tray_theme: 'color',
    check_for_updates: true,
    connection_verification_time: 10,
    peer_alive_period: 300,
  },
};

export const useClientStore = createWithEqualityFn<Store>()(
  persist(
    (set, get) => ({
      ...defaultValues,
      setState: (values) => set({ ...values }),
      setInstances: (values) => {
        if (isUndefined(get().selectedInstance)) {
          return set({
            instances: values,
            selectedInstance: {
              id: values[0]?.id,
              type: WireguardInstanceType.DEFGUARD_INSTANCE,
            },
          });
        }
        return set({ instances: values });
      },
      setTunnels: (values) => {
        if (isUndefined(get().selectedInstance)) {
          return set({
            tunnels: values,
            selectedInstance: { id: values[0]?.id, type: WireguardInstanceType.TUNNEL },
          });
        }
        return set({ tunnels: values });
      },
      setListChecked: async (values: boolean) => {
        return set({ listChecked: values });
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
      updateAppConfig: async (data) => {
        // don't emit event bcs this updates store anyway
        const newConfig = await setAppConfig(data, false);
        set({ appConfig: newConfig });
      },
    }),
    {
      name: 'client-store',
      storage: createJSONStorage(() => localStorage),
      partialize: (store) => pickBy(store, ['selectedView']),
      version: 1,
    },
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  instances: DefguardInstance[];
  tunnels: CommonWireguardFields[];
  statsFilter: number;
  selectedInstance?: SelectedInstance;
  selectedLocation?: number;
  // launch carousel page if there is no instances or/and tunnels for the first time after launching application
  listChecked: boolean;
  selectedView: ClientView;
  appConfig: AppConfig;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  setInstances: (instances: DefguardInstance[]) => void;
  setTunnels: (tunnels: CommonWireguardFields[]) => void;
  setListChecked: (listChecked: boolean) => void;
  updateInstances: () => Promise<void>;
  updateAppConfig: (data: Partial<AppConfig>) => Promise<void>;
};
