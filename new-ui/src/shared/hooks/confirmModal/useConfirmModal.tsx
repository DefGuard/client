import { create } from 'zustand';
import type { ButtonProps } from '../../components/Button/types';

type StoreValues = {
  visible: boolean;
  title: string;
  content?: string | null;
  cancelProps: ButtonProps | null;
  submitProps: ButtonProps | null;
  onSubmit: () => Promise<void>;
};

const emptyPromise = async () => {};

const defaults: StoreValues = {
  visible: false,
  title: 'Confirm action',
  content: null,
  submitProps: null,
  cancelProps: null,
  onSubmit: emptyPromise,
};

interface Store extends StoreValues {
  open: (values: Partial<StoreValues>) => void;
  reset: () => void;
}

export const useConfirmModal = create<Store>()((set) => ({
  ...defaults,
  open: (values) => {
    set({ ...defaults, ...values, visible: true });
  },
  reset: () => {
    set(defaults);
  },
}));
