import { clone, omit } from 'radashi';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { MfaMethodValue } from '../../../../shared/rust-api/types';

type LocationMFAPref = Record<number, MfaMethodValue | undefined | null>;

interface StoreValues {
  selectedInstance: number | null;
  expandedLocation: number | null;
  locationsMfaPref: LocationMFAPref;
}

interface Store extends StoreValues {
  setLocationMfa: (locationId: number, factor: MfaMethodValue | null) => void;
}

export const useCompactLocationStore = create<Store>()(
  persist(
    (set, get) => ({
      selectedInstance: null,
      expandedLocation: null,
      locationsMfaPref: {},
      setLocationMfa: (id, factor) => {
        const toSetCopy = clone(get().locationsMfaPref);
        toSetCopy[id] = factor;
        set({ locationsMfaPref: toSetCopy });
      },
    }),
    {
      name: 'compact-locations-store',
      storage: createJSONStorage(() => localStorage),
      version: 1,
      partialize: (state) => omit(state, ['setLocationMfa']),
    },
  ),
);
