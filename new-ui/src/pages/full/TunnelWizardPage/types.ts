export const TunnelWizardStep = {
  GeneralInformation: 'general-information',
  Keys: 'keys',
  VpnServer: 'vpn-server',
  AdvancedSettings: 'advanced-settings',
  Finish: 'finish',
} as const;

export type TunnelWizardStepValue =
  (typeof TunnelWizardStep)[keyof typeof TunnelWizardStep];
