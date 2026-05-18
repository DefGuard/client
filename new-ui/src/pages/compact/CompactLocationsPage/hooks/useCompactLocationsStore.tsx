import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { InstanceInfo, LocationInfo } from '../../../../shared/rust-api/types';

export type CompactViewSelection =
  | { kind: 'instance'; data: InstanceInfo }
  | { kind: 'tunnel'; data: LocationInfo };

interface StoreValues {
  compactViewSelection: CompactViewSelection | null;
  expandedLocation: number | null;
}

interface Store extends StoreValues {}

export const useCompactLocationStore = create<Store>()(
  persist(
    (_) => ({
      compactViewSelection: null,
      expandedLocation: null,
    }),
    {
      name: 'compact-locations-store',
      storage: createJSONStorage(() => localStorage),
      version: 3,
    },
  ),
);
