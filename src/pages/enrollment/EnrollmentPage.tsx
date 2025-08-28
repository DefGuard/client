import './style.scss';

import { debug, error } from '@tauri-apps/plugin-log';
import dayjs from 'dayjs';
import { useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { LogoContainer } from '../../components/LogoContainer/LogoContainer';
import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { deviceBreakpoints } from '../../shared/constants';
import { ArrowSingle } from '../../shared/defguard-ui/components/icons/ArrowSingle/ArrowSingle';
import {
  ArrowSingleDirection,
  ArrowSingleSize,
} from '../../shared/defguard-ui/components/icons/ArrowSingle/types';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/defguard-ui/components/Layout/Button/types';
import { routes } from '../../shared/routes';
import { EnrollmentSideBar } from './components/EnrollmentSideBar/EnrollmentSideBar';
import { EnrollmentStepControls } from './components/EnrollmentStepControls/EnrollmentStepControls';
import { EnrollmentStepKey, enrollmentSteps, flattenEnrollConf } from './const';
import { useEnrollmentStore } from './hooks/store/useEnrollmentStore';

export const EnrollmentPage = () => {
  const enrollmentFinished = useRef(false);
  const navigate = useNavigate();
  const { LL } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const sessionEnd = useEnrollmentStore((state) => state.sessionEnd);
  const currentStep = useEnrollmentStore((state) => state.step);
  const loading = useEnrollmentStore((state) => state.loading);

  const [back, next] = useEnrollmentStore((state) => [state.back, state.next], shallow);

  const controlsSize: ButtonSize =
    breakpoint !== 'desktop' ? ButtonSize.SMALL : ButtonSize.LARGE;

  const flatConf = useMemo(() => flattenEnrollConf(), []);

  const currentStepConfig = flatConf[currentStep];

  useEffect(() => {
    if (!enrollmentFinished.current) {
      if (sessionEnd) {
        const endDay = dayjs(sessionEnd);
        const diff = endDay.diff(dayjs(), 'millisecond');
        if (diff > 0) {
          const timeout = setTimeout(() => {
            if (!enrollmentFinished.current) {
              debug('Enrollment session time ended, navigating to timeout page.');
              navigate(routes.timeout, { replace: true });
            }
          }, diff);
          return () => {
            clearTimeout(timeout);
          };
        } else {
          debug('Enrollment session time ended, navigating to timeout page.');
          navigate(routes.timeout, { replace: true });
        }
      } else {
        error('Session end time not found, navigating to timeout page.');
        navigate(routes.timeout, { replace: true });
      }
    }
  }, [sessionEnd, navigate]);

  useEffect(() => {
    enrollmentFinished.current = currentStep === EnrollmentStepKey.FINISH;
  }, [currentStep]);

  useEffect(() => {
    console.log(currentStepConfig);
  }, [currentStepConfig]);

  return (
    <PageContainer id="enrollment">
      <EnrollmentSideBar />
      <LogoContainer />
      <EnrollmentStepControls>
        <Button
          text={LL.common.controls.back()}
          size={controlsSize}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => back()}
          disabled={!currentStepConfig?.backEnabled || loading}
          icon={
            <ArrowSingle
              size={ArrowSingleSize.SMALL}
              direction={ArrowSingleDirection.LEFT}
            />
          }
        />
        <Button
          data-testid="enrollment-next"
          disabled={currentStepConfig.nextDisabled}
          loading={loading}
          text={LL.common.controls.next()}
          size={controlsSize}
          styleVariant={ButtonStyleVariant.PRIMARY}
          onClick={() => next()}
          rightIcon={<ArrowSingle size={ArrowSingleSize.SMALL} />}
        />
      </EnrollmentStepControls>
      {enrollmentSteps[currentStep]}
    </PageContainer>
  );
};
