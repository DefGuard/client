import { create } from 'zustand';
import type { EnrollmentErrorCopyEntry } from '../errorCopy';

type StoreValues = {
  visible: boolean;
  title: string;
  message: string;
};

const defaults: StoreValues = {
  visible: false,
  title: '',
  message: '',
};

interface Store extends StoreValues {
  open: (values: EnrollmentErrorCopyEntry) => void;
  close: () => void;
  reset: () => void;
}

export const useEnrollmentErrorModal = create<Store>()((set) => ({
  ...defaults,
  open: (values) => {
    set({ title: values.title, message: values.message, visible: true });
  },
  close: () => {
    set({ visible: false });
  },
  reset: () => {
    set(defaults);
  },
}));
