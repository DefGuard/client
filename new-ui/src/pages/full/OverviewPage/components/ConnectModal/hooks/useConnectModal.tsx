import { create } from 'zustand';
import {
  type LocationInfo,
  MfaMethod,
  type MfaMethodValue,
} from '../../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../../shared/utils/isPresent';
import { resolveMfaStepPlan } from '../../../../../../shared/utils/mfa';
import { type ConnectModalViewValue, mfaMethodToConnectModalView } from './types';

interface StoreValues {
  visible: boolean;
  location: LocationInfo | null;
  view: ConnectModalViewValue | null;
  perviousView: ConnectModalViewValue | null;
  postureError: string | null;
  connectionError: string | null;
  autoStartOpenId: boolean;
  mfaMethod: MfaMethodValue;
  stepIndex: number;
  stepPlan: MfaMethodValue[];
  mfaToken: string | null;
}

const defaults: StoreValues = {
  visible: false,
  mfaMethod: MfaMethod.Totp,
  location: null,
  view: null,
  perviousView: null,
  postureError: null,
  connectionError: null,
  autoStartOpenId: false,
  stepIndex: 0,
  stepPlan: [],
  mfaToken: null,
} as const;

interface Store extends StoreValues {
  open: (init?: Partial<StoreValues>) => void;
  setView: (view: ConnectModalViewValue, values?: Partial<StoreValues>) => void;
  setMfaToken: (token: string) => void;
  goToStep: (stepIndex: number) => void;
  reset: () => void;
}

export const useConnectModal = create<Store>((set, get) => ({
  ...defaults,
  reset: () => {
    set(defaults);
  },
  open: (init) => {
    const location = init?.location ?? null;
    const stepPlan = isPresent(location) ? resolveMfaStepPlan(location) : [];
    set({ ...defaults, ...init, stepPlan, visible: true });
  },
  setMfaToken: (token) => {
    set({ mfaToken: token });
  },
  goToStep: (stepIndex) => {
    const { stepPlan, setView } = get();
    setView(mfaMethodToConnectModalView(stepPlan[stepIndex]), { stepIndex });
  },
  setView: (view, vals) => {
    const pervious = get().view ?? null;
    if (vals) {
      set({ ...vals, view, perviousView: pervious });
    } else {
      set({ view, perviousView: pervious });
    }
  },
}));
