import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

interface StoreValues {
  selectedInstance: number | null;
  expandedLocation: number | null;
}

interface Store extends StoreValues {}

export const useCompactLocationStore = create<Store>()(
  persist(
    (_) => ({
      selectedInstance: null,
      expandedLocation: null,
    }),
    {
      name: 'compact-locations-store',
      storage: createJSONStorage(() => localStorage),
      version: 2,
    },
  ),
);
