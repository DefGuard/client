import { create } from 'zustand';
import {
  type LocationInfo,
  MfaMethod,
  type MfaMethodValue,
} from '../../../../../../shared/rust-api/types';
import type { ConnectModalViewValue } from './types';

interface StoreValues {
  visible: boolean;
  location: LocationInfo | null;
  view: ConnectModalViewValue | null;
  perviousView: ConnectModalViewValue | null;
  postureError: string | null;
  autoStartOpenId: boolean;
  mfaMethod: MfaMethodValue;
}

const defaults: StoreValues = {
  visible: false,
  mfaMethod: MfaMethod.Totp,
  location: null,
  view: null,
  perviousView: null,
  postureError: null,
  autoStartOpenId: false,
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
    set({ ...defaults, ...init, visible: true });
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
