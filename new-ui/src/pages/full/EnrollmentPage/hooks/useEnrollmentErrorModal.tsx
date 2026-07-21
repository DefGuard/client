import { create } from 'zustand';

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
  open: (values: { title: string; message: string }) => void;
  reset: () => void;
}

export const useEnrollmentErrorModal = create<Store>()((set) => ({
  ...defaults,
  open: (values) => {
    set({ title: values.title, message: values.message, visible: true });
  },
  reset: () => {
    set(defaults);
  },
}));
