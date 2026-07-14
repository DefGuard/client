import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

interface StoreValues {
  // only used in compact mode
  expandedLocation: number | null;
  // Location id whose MFA flow should auto-start (e.g. triggered from the tray).
  // Consumed and cleared by the matching location card. Not persisted so a stale
  // trigger cannot fire on the next launch.
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
