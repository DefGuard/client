import { createWithEqualityFn } from 'zustand/traditional';

export interface NotificationStore {
  header: string;
  text: string;
  dismissed: boolean;
  setValues: (values: Partial<NotificationStore>) => void;
}

const defaultState = {
  header: '',
  text: '',
  dismissed: true,
} as NotificationStore;

export const useNotificationStore = createWithEqualityFn<NotificationStore>(
  (set) => ({
    ...defaultState,
    setValues: (values: Partial<NotificationStore>) => set({ ...values }),
  }),
  Object.is,
);
