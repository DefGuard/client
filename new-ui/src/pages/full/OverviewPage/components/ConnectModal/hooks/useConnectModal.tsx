import { create } from 'zustand';
import type { LocationInfo } from '../../../../../../shared/rust-api/types';
import type { ConnectModalViewValue } from './types';

interface StoreValues {
  visible: boolean;
  location: LocationInfo | null;
  view: ConnectModalViewValue | null;
  perviousView: ConnectModalViewValue | null;
  postureError: string | null;
}

const defaults: StoreValues = {
  visible: false,
  location: null,
  view: null,
  perviousView: null,
  postureError: null,
} as const;

interface Store extends StoreValues {
  open: (init?: Partial<StoreValues>) => void;
  setView: (view: ConnectModalViewValue, values?: Partial<StoreValues>) => void;
  reset: () => void;
}

export const useConnectModal = create<Store>((set, get) => ({
  ...defaults,
  reset: () => {
    set(defaults);
  },
  open: (init) => {
    set({ ...init, visible: true });
  },
  setView: (view, vals) => {
    const pervious = get().view ?? null;

    if (vals) {
      set({ ...vals, view, perviousView: pervious });
    } else {
      set({ view, perviousView: pervious });
    }
  },
}));
