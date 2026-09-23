import './style.scss';
import { useMemo } from 'react';
import { isClientConfigurableMethod } from '../../utils/mfa';
import { MfaSelector } from '../LocationCard/components/MfaSelector/MfaSelector';
import { MfaFactorAction, type MfaSettingsSectionProps } from './types';
import { mfaSettingsStepsOf } from './utils';

/** Controlled, state lives in `useMfaSettingsSection`. */
export const MfaSettingsSection = ({
  location,
  instance,
  plan,
  configureMethods,
  stepIndices,
  configurable = false,
  onSelectMethod,
  onToggleConfigureMethod,
}: MfaSettingsSectionProps) => {
  const steps = useMemo(
    () => mfaSettingsStepsOf({ location, instance, stepIndices, configurable }),
    [location, instance, stepIndices, configurable],
  );

  if (steps.length === 0) return null;

  const showStepLabels = steps.length > 1;

  return (
    <div className="mfa-settings-section">
      {steps.map(({ stepIndex, factors }) => (
        <div className="step" key={stepIndex}>
          {showStepLabels && <p className="step-label">Step {stepIndex + 1}</p>}
          <div className="methods">
            {factors.map(({ method, configured, isDefault, action }) => (
              <MfaSelector
                key={method}
                factor={method}
                configured={configured}
                isDefault={isDefault}
                active={action === MfaFactorAction.Pick && plan[stepIndex] === method}
                selected={
                  action === MfaFactorAction.Configure &&
                  configureMethods.some((queued) => queued === method)
                }
                isSelectable={action !== MfaFactorAction.None}
                onClick={() => {
                  if (action === MfaFactorAction.Pick) {
                    onSelectMethod(stepIndex, method);
                  } else if (
                    action === MfaFactorAction.Configure &&
                    isClientConfigurableMethod(method)
                  ) {
                    onToggleConfigureMethod(method);
                  }
                }}
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};
