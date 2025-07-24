import { createWithEqualityFn } from 'zustand/traditional';

import type { Tunnel } from '../../../../types';

const defaultValues: StoreValues = {
  isOpen: false,
  tunnel: undefined,
};

export const useDeleteTunnelModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (tunnel) => set({ tunnel, isOpen: true }),
    close: () => set({ isOpen: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
  tunnel?: Tunnel;
};

type StoreMethods = {
  open: (tunnel: Tunnel) => void;
  close: () => void;
  reset: () => void;
};
