import type { MfaStepMethod } from '../../rust-api/types';
import { isPresent } from '../../utils/isPresent';
import {
  canSetUpMfaMethod,
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

  return configurable && canSetUpMfaMethod(entry.method, instance)
    ? MfaFactorAction.Configure
    : MfaFactorAction.None;
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
  const defaultPlan = resolveMfaStepPlan(location, [], instance);

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
