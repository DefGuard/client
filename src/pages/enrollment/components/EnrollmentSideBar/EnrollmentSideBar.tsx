import './style.scss';

import { getVersion } from '@tauri-apps/api/app';
import clsx from 'clsx';
import dayjs from 'dayjs';
import { Fragment, useCallback, useEffect, useMemo, useState } from 'react';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { Divider } from '../../../../shared/defguard-ui/components/Layout/Divider/Divider.tsx';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent.ts';
import { EnrollmentStepKey, enrollmentStepsConfig } from '../../const.tsx';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { AdminInfo } from '../AdminInfo/AdminInfo';
import { TimeLeft } from '../TimeLeft/TimeLeft';

type SideBarItem = {
  stepKey: EnrollmentStepKey;
  label: string;
  activeKeys: EnrollmentStepKey[];
  children?: SideBarItem[];
};

export const EnrollmentSideBar = () => {
  const { LL } = useI18nContext();

  const vpnOptional = useEnrollmentStore((state) => state.vpnOptional);

  const currentStep = useEnrollmentStore((state) => state.step);

  const [appVersion, setAppVersion] = useState<string | undefined>(undefined);

  const translateStep = useCallback(
    (step: EnrollmentStepKey) => {
      const stepsLL = LL.pages.enrollment.sideBar.steps;
      switch (step) {
        case EnrollmentStepKey.WELCOME:
          return stepsLL.welcome();
        case EnrollmentStepKey.DATA_VERIFICATION:
          return stepsLL.verification();
        case EnrollmentStepKey.PASSWORD:
          return stepsLL.password();
        case EnrollmentStepKey.DEVICE:
          return `${stepsLL.vpn()}${vpnOptional ? '*' : ''}`;
        case EnrollmentStepKey.MFA:
          return stepsLL.mfa();
        case EnrollmentStepKey.MFA_CHOICE:
          return stepsLL.mfaChoice();
        case EnrollmentStepKey.MFA_SETUP:
          return stepsLL.mfaSetup();
        case EnrollmentStepKey.MFA_RECOVERY:
          return stepsLL.mfaRecovery();
        case EnrollmentStepKey.ACTIVATE_USER:
          return '';
        case EnrollmentStepKey.FINISH:
          return stepsLL.finish();
        default:
          return '';
      }
    },
    [LL.pages.enrollment.sideBar.steps, vpnOptional],
  );

  const stepsData = useMemo(
    (): SideBarItem[] =>
      Object.values(enrollmentStepsConfig)
        .filter((item) => !item.hidden)
        .map((item, index) => {
          const res: SideBarItem = {
            label: `${index + 1}. ${translateStep(item.key)}`,
            stepKey: item.key,
            activeKeys: [item.key],
          };
          if (item.children) {
            res.children = item.children
              .filter((child) => !child.hidden)
              .map((child, childIndex) => {
                res.activeKeys.push(child.key);
                const labelPrefix = String.fromCharCode(97 + childIndex);
                return {
                  label: `${labelPrefix}. ${translateStep(child.key)}`,
                  stepKey: child.key,
                  activeKeys: [child.key],
                };
              });
          }
          return res;
        }),
    [translateStep],
  );

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
        {stepsData.map((step) => (
          <Fragment key={step.stepKey}>
            <Step data={step} />
            {isPresent(step.children) &&
              step.children.map((child) => (
                <Step key={child.stepKey} data={child} child />
              ))}
          </Fragment>
        ))}
      </div>
      {currentStep !== EnrollmentStepKey.FINISH && (
        <>
          <TimeLeft />
          <Divider />
        </>
      )}
      {currentStep === EnrollmentStepKey.FINISH && <Divider className="push" />}
      <AdminInfo />
      <Divider />
      <div className="copyright">
        <p>
          Copyright © {`${dayjs().year()} `}
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
  data: SideBarItem;
  child?: boolean;
};

const Step = ({ data, child = false }: StepProps) => {
  const currentStep = useEnrollmentStore((state) => state.step);

  const active = data.activeKeys.includes(currentStep);

  return (
    <div
      className={clsx('step', {
        active,
        child,
      })}
    >
      <p>{data.label}</p>
      {active && !isPresent(data.children) && <div className="active-step-line"></div>}
    </div>
  );
};
