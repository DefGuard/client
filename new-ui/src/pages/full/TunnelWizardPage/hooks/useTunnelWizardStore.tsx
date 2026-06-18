import { create } from 'zustand';
import { TunnelWizardStep, type TunnelWizardStepValue } from '../types';

type StoreValues = {
  activeStep: TunnelWizardStepValue;
};

const defaults: StoreValues = {
  activeStep: TunnelWizardStep.GeneralInformation,
};

const STEPS: TunnelWizardStepValue[] = [
  TunnelWizardStep.GeneralInformation,
  TunnelWizardStep.Keys,
  TunnelWizardStep.VpnServer,
  TunnelWizardStep.AdvancedSettings,
  TunnelWizardStep.Finish,
];

interface Store extends StoreValues {
  next: () => void;
  back: () => void;
  reset: () => void;
}

export const useTunnelWizardStore = create<Store>()((set, get) => ({
  ...defaults,
  next: () => {
    const idx = STEPS.indexOf(get().activeStep);
    if (idx < STEPS.length - 1) set({ activeStep: STEPS[idx + 1] });
  },
  back: () => {
    const idx = STEPS.indexOf(get().activeStep);
    if (idx > 0) set({ activeStep: STEPS[idx - 1] });
  },
  reset: () => set(defaults),
}));
