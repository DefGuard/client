import { pick } from 'lodash-es';
import { Subject } from 'rxjs';
import { createJSONStorage, devtools, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { AdminInfo, UserInfo } from '../../../../shared/hooks/api/types';

const defaultValues: StoreValues = {
  // assume default dev
  proxy_url: '/api/v1/',
  loading: false,
  step: 0,
  stepsMax: 4,
  sessionStart: undefined,
  sessionEnd: undefined,
  userInfo: undefined,
  deviceName: undefined,
  vpnOptional: undefined,
  userPassword: undefined,
  cookie: undefined,
  nextSubject: new Subject<void>(),
};

const persistKeys: Array<keyof StoreValues> = [
  'step',
  'userInfo',
  'userPassword',
  'sessionEnd',
  'sessionStart',
  'adminInfo',
  'deviceName',
  'endContent',
  'vpnOptional',
];

export const useEnrollmentStore = createWithEqualityFn<Store>()(
  devtools(
    persist(
      (set, get) => ({
        ...defaultValues,
        init: (values) => set({ ...defaultValues, ...values }),
        setState: (newValues) => set((old) => ({ ...old, ...newValues })),
        reset: () => set(defaultValues),
        nextStep: () => {
          const current = get().step;
          const max = get().stepsMax;

          if (current < max) {
            return set({ step: current + 1 });
          }
        },
        perviousStep: () => {
          const current = get().step;

          if (current > 0) {
            return set({ step: current - 1 });
          }
        },
      }),
      {
        name: 'enrollment-storage',
        version: 0.1,
        storage: createJSONStorage(() => sessionStorage),
        partialize: (state) => pick(state, persistKeys),
      },
    ),
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  // next and back are disabled
  loading: boolean;
  step: number;
  stepsMax: number;
  nextSubject: Subject<void>;
  // Date
  proxy_url: string;
  sessionStart?: string;
  sessionEnd?: string;
  userInfo?: UserInfo;
  userPassword?: string;
  adminInfo?: AdminInfo;
  vpnOptional?: boolean;
  // Markdown content for final step card
  endContent?: string;
  deviceName?: string;
  cookie?: string;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  reset: () => void;
  nextStep: () => void;
  perviousStep: () => void;
  init: (initValues: Partial<StoreValues>) => void;
};
