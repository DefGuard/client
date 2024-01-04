import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

const defaults: StoreValues = {
  firstStart: true,
};

/*Flags that are persisted via localstorage and are not used by rust backend*/
export const useClientFlags = createWithEqualityFn<Store>()(
  persist(
    (set) => ({
      ...defaults,
      setValues: (vals) => set({ ...vals }),
    }),
    {
      name: 'client-flags',
      version: 1,
      storage: createJSONStorage(() => localStorage),
    },
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  // Is user launching app first time ?
  firstStart: boolean;
};

type StoreMethods = {
  setValues: (values: Partial<StoreValues>) => void;
};
