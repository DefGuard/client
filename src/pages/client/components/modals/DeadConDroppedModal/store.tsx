import { createWithEqualityFn } from 'zustand/traditional';

import type { DeadConDroppedPayload } from '../../../types';

const defaultValues: StoreValues = {
  visible: false,
  payload: undefined,
};

export const useDeadConDroppedModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (val) => set({ visible: true, payload: val }),
    close: () => set({ visible: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreMethods & StoreValues;

type StoreMethods = {
  open: (payload: DeadConDroppedPayload) => void;
  close: () => void;
  reset: () => void;
};

type StoreValues = {
  visible: boolean;
  payload?: DeadConDroppedPayload;
};
