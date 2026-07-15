import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

interface StoreValues {
  // only used in compact mode
  expandedLocation: number | null;
}

interface Store extends StoreValues {}

export const useAppStore = create<Store>()(
  persist(
    (_) => ({
      expandedLocation: null,
    }),
    {
      name: 'app-store',
      storage: createJSONStorage(() => localStorage),
      version: 4,
    },
  ),
);
