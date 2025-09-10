import { createWithEqualityFn } from 'zustand/traditional';

import type { DefguardInstance } from '../../../../types';

const defaultValues: StoreValues = {
  isOpen: false,
  instance: undefined,
};

export const useDeleteInstanceModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (instance) => set({ instance, isOpen: true }),
    close: () => set({ isOpen: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
  instance?: DefguardInstance;
};

type StoreMethods = {
  open: (instance: DefguardInstance) => void;
  close: () => void;
  reset: () => void;
};
