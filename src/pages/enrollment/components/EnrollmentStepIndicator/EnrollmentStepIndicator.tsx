import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

export const EnrollmentStepIndicator = () => {
  const { LL } = useI18nContext();

  const [step, maxStep] = useEnrollmentStore((state) => [
    state.step + 1,
    state.stepsMax - state.stepsIgnored.length + 1,
  ]);

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
