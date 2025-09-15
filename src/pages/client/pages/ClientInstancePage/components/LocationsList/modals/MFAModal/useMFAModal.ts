import { createWithEqualityFn } from 'zustand/traditional';

import type { CommonWireguardFields } from '../../../../../../types';

const defaultValues: StoreValues = {
  isOpen: false,
  instance: undefined,
};

export const useMFAModal = createWithEqualityFn<Store>(
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
  instance?: CommonWireguardFields;
};

type StoreMethods = {
  open: (instance: CommonWireguardFields) => void;
  close: () => void;
  reset: () => void;
};
