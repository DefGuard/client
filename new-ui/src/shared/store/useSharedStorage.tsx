import { createTauriStore } from '@tauri-store/zustand';
import { clone } from 'radashi';
import { create, type StoreApi } from 'zustand';
import {
  type InstanceInfo,
  type LocationInfo,
  MfaMethod,
  type MfaMethodValue,
} from '../rust-api/types';

export type CompactViewSelection =
  | { kind: 'instance'; data: InstanceInfo }
  | { kind: 'tunnel'; data: LocationInfo };

interface StoreValues {
  locationMethodSelection: Record<number, MfaMethodValue | undefined>;
  viewSelection: CompactViewSelection | null;
}

interface Store extends StoreValues {
  setLocationMethod: (id: number, method: MfaMethodValue) => void;
  getLocationMethod: (id: number) => MfaMethodValue;
}

export const useSharedStorage = create<Store>()((set, get) => ({
  locationMethodSelection: {},
  viewSelection: null,
  getLocationMethod: (locationId) => {
    const selection = get().locationMethodSelection;
    return selection[locationId] ?? MfaMethod.Totp;
  },
  setLocationMethod: (locationId, method) => {
    const selection = clone(get().locationMethodSelection);
    selection[locationId] = method;
    set({ locationMethodSelection: selection });
  },
}));

export const sharedStorageTauriHandler = createTauriStore(
  'shared-session-store',
  useSharedStorage as unknown as StoreApi<Record<string, unknown>>,
  { autoStart: true, syncStrategy: 'debounce', syncInterval: 500 },
);
