import './style.scss';

import { getVersion } from '@tauri-apps/api/app';
import classNames from 'classnames';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { Divider } from '../../../../shared/defguard-ui/components/Layout/Divider/Divider.tsx';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { AdminInfo } from '../AdminInfo/AdminInfo';
import { TimeLeft } from '../TimeLeft/TimeLeft';
import type { EnrollmentSideBarData } from '../types.ts';

export const EnrollmentSideBar = () => {
  const { LL } = useI18nContext();

  const vpnOptional = useEnrollmentStore((state) => state.vpnOptional);

  const [currentStep, stepsMax] = useEnrollmentStore((state) => [
    state.step,
    state.stepsMax,
  ]);

  const [appVersion, setAppVersion] = useState<string | undefined>(undefined);

  const steps = useMemo((): EnrollmentSideBarData[] => {
    const steps = LL.pages.enrollment.sideBar.steps;
    const vpnStep = vpnOptional ? `${steps.vpn()}*` : steps.vpn();
    return [
      {
        label: steps.welcome(),
        stepDisplayNumber: 1,
        stepIndex: 0,
      },
      {
        label: steps.verification(),
        stepDisplayNumber: 2,
        stepIndex: 1,
      },
      {
        label: steps.password(),
        stepDisplayNumber: 3,
        stepIndex: 2,
      },
      {
        label: vpnStep,
        stepDisplayNumber: 4,
        stepIndex: 3,
      },
      {
        label: `${steps.mfa()}*`,
        stepDisplayNumber: 5,
        stepIndex: 4,
      },
      {
        label: steps.finish(),
        stepDisplayNumber: 6,
        stepIndex: 5,
      },
    ];
  }, [LL.pages.enrollment.sideBar.steps, vpnOptional]);

  useEffect(() => {
    const getAppVersion = async () => {
      const version = await getVersion().catch(() => {
        return '';
      });
      setAppVersion(version);
    };

    getAppVersion();
  }, []);

  return (
    <div id="enrollment-side-bar">
      <div className="title">
        <h2>{LL.pages.enrollment.sideBar.title()}</h2>
      </div>
      <Divider />
      <div className="steps">
        {steps.map((data) => (
          <Step
            data={data}
            key={Array.isArray(data.stepIndex) ? data.stepIndex[0] : data.stepIndex}
          />
        ))}
      </div>
      {currentStep !== stepsMax && (
        <>
          <TimeLeft />
          <Divider />
        </>
      )}
      {currentStep === stepsMax && <Divider className="push" />}
      <AdminInfo />
      <Divider />
      <div className="copyright">
        <p>
          Copyright © {`${dayjs().year}`}
          <a href="https://defguard.net" target="_blank" rel="noopener noreferrer">
            defguard
          </a>
        </p>
        <p>
          {LL.pages.enrollment.sideBar.appVersion()}: {appVersion}
        </p>
      </div>
    </div>
  );
};

type StepProps = {
  data: EnrollmentSideBarData;
};

const Step = ({ data: { label, stepIndex, stepDisplayNumber } }: StepProps) => {
  const currentStep = useEnrollmentStore((state) => state.step);

  const active = Array.isArray(stepIndex)
    ? stepIndex.includes(currentStep)
    : stepIndex === currentStep;

  const cn = classNames('step', {
    active,
  });

  return (
    <div className={cn}>
      <p>
        {stepDisplayNumber}.{'  '}
        {label}
      </p>
      {active && <div className="active-step-line"></div>}
    </div>
  );
};
