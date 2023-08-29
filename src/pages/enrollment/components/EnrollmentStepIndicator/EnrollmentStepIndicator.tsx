import './style.scss';

import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

export const EnrollmentStepIndicator = () => {
  const { LL } = useI18nContext();

  const [step, maxStep] = useEnrollmentStore(
    (state) => [state.step, state.stepsMax],
    shallow,
  );

  return (
    <div className="step-indicator">
      <p>
        {LL.pages.enrollment.stepsIndicator.step()} {step + 1}{' '}
        <span>
          {LL.pages.enrollment.stepsIndicator.of()} {maxStep + 1}
        </span>
      </p>
    </div>
  );
};
