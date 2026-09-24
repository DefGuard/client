import type { InstanceInfo, LocationInfo, MfaMethodValue } from '../../rust-api/types';
import type { ClientConfigurableMethod } from '../../utils/mfa';

export type MfaSettingsLocation = Pick<
  LocationInfo,
  'connection_type' | 'mfa_steps' | 'mfa_step_plan'
>;

export type MfaSettingsInstance = Pick<InstanceInfo, 'mfa_configured_methods'>;

/** What clicking a factor row does. */
export const MfaFactorAction = {
  /** Configured and drivable here, sets the step's active factor. */
  Pick: 'pick',
  /** Not configured but settable here, queues it for configuration. */
  Configure: 'configure',
  /** Listed for context only. */
  None: 'none',
} as const;

export type MfaFactorActionValue = (typeof MfaFactorAction)[keyof typeof MfaFactorAction];

export type MfaSettingsFactor = {
  method: MfaMethodValue;
  configured: boolean;
  isDefault: boolean;
  action: MfaFactorActionValue;
};

export type MfaSettingsStep = {
  stepIndex: number;
  factors: MfaSettingsFactor[];
};

export interface MfaSettingsSectionProps {
  location: MfaSettingsLocation;
  instance?: MfaSettingsInstance;
  /** Active factor per location step. */
  plan: MfaMethodValue[];
  /** Queued for a configuration session. */
  configureMethods: ClientConfigurableMethod[];
  /** Steps to render, all when omitted. */
  stepIndices?: number[];
  /** Allows queueing unconfigured factors for setup. */
  configurable?: boolean;
  onSelectMethod: (stepIndex: number, method: MfaMethodValue) => void;
  onToggleConfigureMethod: (method: ClientConfigurableMethod) => void;
}
