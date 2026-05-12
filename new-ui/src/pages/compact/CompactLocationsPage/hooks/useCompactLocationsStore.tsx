import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

interface StoreValues {
  selectedInstance: number | null;
}

interface Store extends StoreValues {}

export const useCompactLocationStore = create<Store>()(
  persist<Store>(
    () => ({
      selectedInstance: null,
    }),
    {
      name: 'compact-locations-store',
      storage: createJSONStorage(() => localStorage),
      version: 1,
    },
  ),
);
