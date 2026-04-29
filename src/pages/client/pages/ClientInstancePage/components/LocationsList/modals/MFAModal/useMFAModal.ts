import { createWithEqualityFn } from 'zustand/traditional';

import type { CommonWireguardFields } from '../../../../../../types';

const defaultValues: StoreValues = {
  isOpen: false,
  instance: undefined,
  autoConnect: false,
};

export const useMFAModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (instance, autoConnect = false) => set({ instance, isOpen: true, autoConnect }),
    close: () => set({ isOpen: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
  instance?: CommonWireguardFields;
  autoConnect: boolean;
};

type StoreMethods = {
  open: (instance: CommonWireguardFields, autoConnect?: boolean) => void;
  close: () => void;
  reset: () => void;
};
