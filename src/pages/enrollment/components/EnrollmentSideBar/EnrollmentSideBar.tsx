import './style.scss';

import classNames from 'classnames';
import { useEffect, useMemo, useState } from 'react';
import { LocalizedString } from 'typesafe-i18n';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Divider } from '../../../../shared/defguard-ui/components/Layout/Divider/Divider.tsx';
import { useApi } from '../../../../shared/hooks/api/useApi.tsx';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { AdminInfo } from '../AdminInfo/AdminInfo';
import { TimeLeft } from '../TimeLeft/TimeLeft';

export const EnrollmentSideBar = () => {
  const { LL } = useI18nContext();

  const vpnOptional = useEnrollmentStore((state) => state.vpnOptional);

  // fetch app version
  const { getAppInfo } = useApi();
  const [appVersion, setAppVersion] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (!appVersion) {
      getAppInfo()
        .then((res) => {
          setAppVersion(res.version);
        })
        .catch((err) => {
          console.error('Failed to fetch app info: ', err);
        });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const steps = useMemo((): LocalizedString[] => {
    const steps = LL.pages.enrollment.sideBar.steps;
    const vpnStep = vpnOptional ? `${steps.vpn()}*` : steps.vpn();
    return [
      steps.welcome(),
      steps.verification(),
      steps.password(),
      vpnStep as LocalizedString,
      steps.finish(),
    ];
  }, [LL.pages.enrollment.sideBar.steps, vpnOptional]);

  return (
    <div id="enrollment-side-bar">
      <div className="title">
        <h2>{LL.pages.enrollment.sideBar.title()}</h2>
      </div>
      <Divider />
      <div className="steps">
        {steps.map((text, index) => (
          <Step text={text} index={index} key={index} />
        ))}
      </div>
      <TimeLeft />
      <Divider />
      <AdminInfo />
      <Divider />
      <div className="copyright">
        <p>
          Copyright © 2023{' '}
          <a href="https://teonite.com" target="_blank" rel="noopener noreferrer">
            teonite
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
  text: LocalizedString;
  index: number;
};

const Step = ({ index, text }: StepProps) => {
  const currentStep = useEnrollmentStore((state) => state.step);

  const active = currentStep === index;

  const cn = classNames('step', {
    active,
  });

  return (
    <div className={cn}>
      <p>
        {index + 1}.{'  '}
        {text}
      </p>
      {active && <div className="active-step-line"></div>}
    </div>
  );
};
