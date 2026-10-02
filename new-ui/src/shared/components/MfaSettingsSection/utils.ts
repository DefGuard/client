import type { MfaStepMethod } from '../../rust-api/types';
import { isPresent } from '../../utils/isPresent';
import {
  isClientConfigurableMethod,
  isDesktopDrivableMethod,
  isMfaMethodConfigured,
  isMfaMethodUsable,
  mfaStepsOf,
  pickableMfaMethods,
  resolveMfaStepPlan,
} from '../../utils/mfa';
import {
  MfaFactorAction,
  type MfaFactorActionValue,
  type MfaSettingsInstance,
  type MfaSettingsLocation,
  type MfaSettingsStep,
} from './types';

const mfaFactorActionOf = (
  entry: MfaStepMethod,
  instance: MfaSettingsInstance | undefined,
  configurable: boolean,
): MfaFactorActionValue => {
  if (isMfaMethodUsable(entry, instance)) return MfaFactorAction.Pick;

  // Like `connectionAbilityOf`, an instance without reported factors cannot configure.
  const canConfigure =
    configurable &&
    isPresent(instance?.mfa_configured_methods) &&
    isDesktopDrivableMethod(entry.method) &&
    isClientConfigurableMethod(entry.method);

  return canConfigure ? MfaFactorAction.Configure : MfaFactorAction.None;
};

export const mfaSettingsStepsOf = ({
  location,
  instance,
  stepIndices,
  configurable = false,
}: {
  location: MfaSettingsLocation;
  instance?: MfaSettingsInstance;
  stepIndices?: number[];
  configurable?: boolean;
}): MfaSettingsStep[] => {
  const defaultPlan = resolveMfaStepPlan(location);

  return mfaStepsOf(location)
    .map((step, stepIndex) => ({
      stepIndex,
      factors: pickableMfaMethods(step).map((entry) => ({
        method: entry.method,
        configured: isMfaMethodConfigured(entry, instance),
        isDefault: defaultPlan[stepIndex] === entry.method,
        action: mfaFactorActionOf(entry, instance, configurable),
      })),
    }))
    .filter(
      ({ stepIndex }) => !isPresent(stepIndices) || stepIndices.includes(stepIndex),
    );
};
