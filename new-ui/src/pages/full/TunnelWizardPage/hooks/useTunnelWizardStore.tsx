import { create } from 'zustand';
import { TunnelWizardStep, type TunnelWizardStepValue } from '../types';

type StoreValues = {
  activeStep: TunnelWizardStepValue;
  tunnelData: {
    name: string;
    pubkey: string;
    prvkey: string;
    address: string;
    server_pubkey: string;
    preshared_key: string;
    allowed_ips?: string;
    endpoint: string;
    dns?: string;
    persistent_keep_alive: number;
    route_all_traffic: boolean;
    pre_up?: string;
    post_up?: string;
    pre_down?: string;
    post_down?: string;
  };
};

const defaults: StoreValues = {
  activeStep: TunnelWizardStep.GeneralInformation,
  tunnelData: {
    name: '',
    address: '',
    endpoint: '',
    persistent_keep_alive: 25,
    preshared_key: '',
    prvkey: '',
    pubkey: '',
    route_all_traffic: false,
    server_pubkey: '',
    allowed_ips: '',
    dns: '',
    post_down: '',
    post_up: '',
    pre_down: '',
    pre_up: '',
  },
};

const nextStep = (step: TunnelWizardStepValue): TunnelWizardStepValue => {
  switch (step) {
    case TunnelWizardStep.GeneralInformation:
      return TunnelWizardStep.Keys;
    case TunnelWizardStep.Keys:
      return TunnelWizardStep.VpnServer;
    case TunnelWizardStep.VpnServer:
      return TunnelWizardStep.AdvancedSettings;
    case TunnelWizardStep.AdvancedSettings:
      return TunnelWizardStep.Finish;
    default:
      return step;
  }
};

const prevStep = (step: TunnelWizardStepValue): TunnelWizardStepValue => {
  switch (step) {
    case TunnelWizardStep.Keys:
      return TunnelWizardStep.GeneralInformation;
    case TunnelWizardStep.VpnServer:
      return TunnelWizardStep.Keys;
    case TunnelWizardStep.AdvancedSettings:
      return TunnelWizardStep.VpnServer;
    case TunnelWizardStep.Finish:
      return TunnelWizardStep.AdvancedSettings;
    default:
      return step;
  }
};

interface Store extends StoreValues {
  next: (values?: Partial<StoreValues['tunnelData']>) => void;
  back: (values?: Partial<StoreValues['tunnelData']>) => void;
  reset: () => void;
}

export const useTunnelWizardStore = create<Store>()((set, get) => ({
  ...defaults,
  next: (tunnelData) => {
    set({
      activeStep: nextStep(get().activeStep),
      tunnelData: { ...get().tunnelData, ...tunnelData },
    });
  },
  back: (tunnelData) => {
    set({
      activeStep: prevStep(get().activeStep),
      tunnelData: { ...get().tunnelData, ...tunnelData },
    });
  },
  reset: () => set(defaults),
}));
