import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  isOpen: false,
};

export const useAddInstanceModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (initial) => set({ ...defaultValues, ...initial, isOpen: true }),
    close: () => set({ isOpen: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
};

type StoreMethods = {
  open: (init?: Partial<StoreValues>) => void;
  close: () => void;
  reset: () => void;
};
