import { createWithEqualityFn } from 'zustand/traditional';
import type { AddInstanceInitResponse } from '../components/AddInstanceFormCard/types';
import { AddInstanceFormStep } from './types';

const defaults: StoreValues = {
  step: AddInstanceFormStep.INIT,
  response: undefined,
};

export const useAddInstanceStore = createWithEqualityFn<Store>((set) => ({
  ...defaults,
  setState: (values) => set((old) => ({ ...old, ...values })),
  reset: () => set(defaults),
}));

type Store = StoreMethods & StoreValues;

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  reset: () => void;
};

type StoreValues = {
  step: AddInstanceFormStep;
  response?: AddInstanceInitResponse;
};
