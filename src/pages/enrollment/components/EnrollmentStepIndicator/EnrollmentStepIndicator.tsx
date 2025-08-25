import './style.scss';

import { useMemo } from 'react';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

export const EnrollmentStepIndicator = () => {
  const { LL } = useI18nContext();

  const [stateStep, maxStep, ignoredSteps] = useEnrollmentStore((state) => [
    state.step,
    state.stepsMax - state.stepsIgnored.length + 1,
    state.stepsIgnored,
  ]);

  const step = useMemo(() => {
    let res = stateStep;
    ignoredSteps.forEach((ignored) => {
      if (ignored <= stateStep) {
        res += 1;
      }
    });
    return res + 1;
  }, [ignoredSteps, stateStep]);

  return (
    <div className="step-indicator">
      <p>
        {LL.pages.enrollment.stepsIndicator.step()} {step}{' '}
        <span>
          {LL.pages.enrollment.stepsIndicator.of()} {maxStep}
        </span>
      </p>
    </div>
  );
};
