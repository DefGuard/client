import './style.scss';

import { useMemo } from 'react';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { flattenEnrollConf } from '../../const';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

export const EnrollmentStepIndicator = () => {
  const { LL } = useI18nContext();

  const currentStepKey = useEnrollmentStore((state) => state.step);
  const flatConf = useMemo(() => flattenEnrollConf(), []);
  const currentStep = flatConf[currentStepKey];

  return (
    <div className="step-indicator">
      <p>
        {LL.pages.enrollment.stepsIndicator.step()}{' '}
        {currentStep.indicatorPrefix ?? currentStep.sideBarPrefix ?? ''}{' '}
        <span>{LL.pages.enrollment.stepsIndicator.of()} 6</span>
      </p>
    </div>
  );
};
