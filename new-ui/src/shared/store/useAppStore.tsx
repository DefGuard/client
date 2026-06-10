import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { InstanceInfo, LocationInfo } from '../rust-api/types';

export type CompactViewSelection =
  | { kind: 'instance'; data: InstanceInfo }
  | { kind: 'tunnel'; data: LocationInfo };

interface StoreValues {
  compactViewSelection: CompactViewSelection | null;
  expandedLocation: number | null;
}

interface Store extends StoreValues {}

export const useAppStore = create<Store>()(
  persist(
    (_) => ({
      compactViewSelection: null,
      expandedLocation: null,
    }),
    {
      name: 'app-store',
      storage: createJSONStorage(() => localStorage),
      version: 3,
    },
  ),
);
