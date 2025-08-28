import { pick } from 'lodash-es';
import { Subject } from 'rxjs';
import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';
import type {
  AdminInfo,
  CreateDeviceResponse,
  UserInfo,
} from '../../../../shared/hooks/api/types';
import { MfaMethod } from '../../../../shared/types';
import { EnrollmentStepKey } from '../../const';
import { EnrollmentNavDirection } from '../types';

const defaultValues: StoreValues = {
  // assume default dev
  proxy_url: '/api/v1/',
  loading: false,
  step: EnrollmentStepKey.WELCOME,
  mfaMethod: MfaMethod.TOTP,
  recoveryCodes: [],
  sessionStart: undefined,
  sessionEnd: undefined,
  userInfo: undefined,
  deviceName: undefined,
  vpnOptional: undefined,
  userPassword: undefined,
  cookie: undefined,
  nextSubject: new Subject(),
  deviceKeys: undefined,
  deviceResponse: undefined,
};

const persistKeys: Array<keyof StoreValues> = [
  'step',
  'proxy_url',
  'userInfo',
  'userPassword',
  'recoveryCodes',
  'mfaMethod',
  'sessionEnd',
  'sessionStart',
  'adminInfo',
  'deviceName',
  'endContent',
  'vpnOptional',
  'deviceKeys',
  'deviceResponse',
  'cookie',
];

export const useEnrollmentStore = createWithEqualityFn<Store>()(
  persist(
    (set, get) => ({
      ...defaultValues,
      init: (values) => set({ ...defaultValues, ...values }),
      setState: (newValues) => set((old) => ({ ...old, ...newValues })),
      reset: () => set(defaultValues),
      next: () => {
        get().nextSubject.next(EnrollmentNavDirection.NEXT);
      },
      back: () => {
        get().nextSubject.next(EnrollmentNavDirection.BACK);
      },
    }),
    {
      name: 'enrollment-storage',
      version: 2,
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) => pick(state, persistKeys),
    },
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  // next and back are disabled
  loading: boolean;
  step: EnrollmentStepKey;
  mfaMethod: MfaMethod;
  nextSubject: Subject<EnrollmentNavDirection>;
  // Date
  proxy_url: string;
  recoveryCodes: string[];
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
  deviceKeys?: {
    public: string;
    private: string;
  };
  deviceResponse?: CreateDeviceResponse;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  init: (initValues: Partial<StoreValues>) => void;
  next: () => void;
  back: () => void;
  reset: () => void;
};
