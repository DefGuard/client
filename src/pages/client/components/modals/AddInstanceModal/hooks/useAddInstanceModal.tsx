import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  isOpen: false,
  loading: false,
  step: 0,
  proxyUrl: undefined,
};

export const useAddInstanceModal = createWithEqualityFn<Store>(
  (set, get) => ({
    ...defaultValues,
    open: (initial) => set({ ...defaultValues, ...initial, isOpen: true }),
    next: (values) => set({ ...values, step: get().step + 1 }),
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
  step: number;
  proxyUrl?: string;
};

type StoreMethods = {
  open: (init?: Partial<StoreValues>) => void;
  next: (values?: Partial<StoreValues>) => void;
  close: () => void;
  reset: () => void;
  setState: (values: Partial<StoreValues>) => void;
};
