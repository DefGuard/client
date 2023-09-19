import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  isOpen: false,
  loading: false,
};

export const useAddInstanceModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (initial) => set({ ...defaultValues, ...initial, isOpen: true }),
    close: () => set({ isOpen: false }),
    reset: () => set(defaultValues),
    setState: (values) => set(values),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
  loading: boolean;
};

type StoreMethods = {
  open: (init?: Partial<StoreValues>) => void;
  close: () => void;
  reset: () => void;
  setState: (values: Partial<StoreValues>) => void;
};
