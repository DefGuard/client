import './style.scss';

import { useMemo } from 'react';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

function getDisplayStep(
  currentIndex: number,
  stepsMax: number,
  ignoredSteps: number[],
): number {
  const ignored = [...new Set(ignoredSteps)]
    .filter((s) => s >= 0 && s < stepsMax)
    .sort((a, b) => a - b);
  const ignoredBefore = ignored.filter((step) => step <= currentIndex).length;
  return currentIndex + 1 - ignoredBefore;
}

export const EnrollmentStepIndicator = () => {
  const { LL } = useI18nContext();

  const [stateStep, maxStep, ignoredSteps] = useEnrollmentStore((state) => [
    state.step,
    state.stepsMax,
    state.stepsIgnored,
  ]);

  const step = useMemo(
    () => getDisplayStep(stateStep, maxStep, ignoredSteps),
    [ignoredSteps, stateStep, maxStep],
  );

  return (
    <div className="step-indicator">
      <p>
        {LL.pages.enrollment.stepsIndicator.step()} {step}{' '}
        <span>
          {LL.pages.enrollment.stepsIndicator.of()} {maxStep + 1 - ignoredSteps.length}
        </span>
      </p>
    </div>
  );
};
