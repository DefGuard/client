import { useCallback, useState } from 'react';
import type { MfaMethodValue } from '../../rust-api/types';
import {
  CLIENT_CONFIGURABLE_METHODS,
  type ClientConfigurableMethod,
  resolveMfaStepPlan,
} from '../../utils/mfa';
import type {
  MfaSettingsInstance,
  MfaSettingsLocation,
  MfaSettingsSectionProps,
} from './types';

interface Options {
  location: MfaSettingsLocation;
  instance?: MfaSettingsInstance;
  /** Defaults to the location's resolved default plan. */
  initialPlan?: MfaMethodValue[];
  stepIndices?: number[];
  configurable?: boolean;
}

/** Spread `sectionProps` onto the section, read back `plan` and `configureMethods`. */
export const useMfaSettingsSection = ({
  location,
  instance,
  initialPlan,
  stepIndices,
  configurable = false,
}: Options) => {
  const [plan, setPlan] = useState<MfaMethodValue[]>(
    () => initialPlan ?? resolveMfaStepPlan(location),
  );
  const [configureMethods, setConfigureMethods] = useState<ClientConfigurableMethod[]>(
    [],
  );

  const selectMethod = useCallback((stepIndex: number, method: MfaMethodValue) => {
    setPlan((current) => {
      const next = [...current];
      next[stepIndex] = method;
      return next;
    });
  }, []);

  const toggleConfigureMethod = useCallback((method: ClientConfigurableMethod) => {
    setConfigureMethods((current) => {
      const isQueued = current.includes(method);
      // Wizard order, not click order.
      return CLIENT_CONFIGURABLE_METHODS.filter((candidate) =>
        candidate === method ? !isQueued : current.includes(candidate),
      );
    });
  }, []);

  const clearConfigureMethods = useCallback(() => setConfigureMethods([]), []);

  const sectionProps: MfaSettingsSectionProps = {
    location,
    instance,
    plan,
    configureMethods,
    stepIndices,
    configurable,
    onSelectMethod: selectMethod,
    onToggleConfigureMethod: toggleConfigureMethod,
  };

  return {
    plan,
    setPlan,
    configureMethods,
    selectMethod,
    toggleConfigureMethod,
    clearConfigureMethods,
    sectionProps,
  };
};
