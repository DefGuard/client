import { type HTMLProps, type PropsWithChildren, Suspense, useMemo } from 'react';
import './style.scss';
import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import Skeleton from 'react-loading-skeleton';
import { ThemeSpacing } from '../../../types';
import { SizedBox } from '../../SizedBox/SizedBox';
import type { WizardPageConfig } from '../types';
import { WizardStepsCard } from '../WizardStepsCard/WizardStepsCard';

type Props = PropsWithChildren &
  WizardPageConfig & {
    className?: string;
    containerProps?: Omit<HTMLProps<HTMLDivElement>, 'className'>;
  };

export const WizardPage = ({
  className,
  activeStep,
  steps,
  title,
  children,
  containerProps,
}: Props) => {
  const activeStepData = steps[activeStep];

  const visibleSteps = useMemo(
    () =>
      orderBy(
        Object.values(steps).filter((step) => !step.hidden),
        (s) => s.order,
        ['asc'],
      ),
    [steps],
  );

  const activeStepIndex = useMemo(
    () => visibleSteps.findIndex((s) => s.id === activeStep),
    [visibleSteps, activeStep],
  );

  return (
    <div className={clsx('wizard-page', className)} {...containerProps}>
      <div className="page-grid">
        <div className="side">
          <p className="title">{title}</p>
          <SizedBox height={ThemeSpacing.Xl} />
          <WizardStepsCard steps={visibleSteps} activeStep={activeStepData} />
        </div>
        <div className="main">
          <div className="wizard-step-badge">
            <p>{`Step ${activeStepIndex + 1} of ${visibleSteps.length}`}</p>
          </div>
          <SizedBox height={ThemeSpacing.Lg} />
          <Suspense fallback={<WizardStepSkeleton />}>{children}</Suspense>
        </div>
      </div>
    </div>
  );
};

const WizardStepSkeleton = () => {
  return (
    <Skeleton containerClassName="wizard-step-skeleton" width={`100%`} height={770} />
  );
};
