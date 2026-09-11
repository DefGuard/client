import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

interface StoreValues {
  // only used in compact mode
  expandedLocation: number | null;
  // Location ID whose MFA flow should auto-start (e.g. triggered from the tray).
  mfaAutoStartLocationId: number | null;
}

interface Store extends StoreValues {}

export const useAppStore = create<Store>()(
  persist(
    (_) => ({
      expandedLocation: null,
      mfaAutoStartLocationId: null,
    }),
    {
      name: 'app-store',
      storage: createJSONStorage(() => localStorage),
      version: 4,
      partialize: (state) => ({ expandedLocation: state.expandedLocation }),
    },
  ),
);
