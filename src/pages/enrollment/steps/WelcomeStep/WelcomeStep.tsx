import './style.scss';

import dayjs from 'dayjs';
import { useEffect, useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeSanitaze from 'rehype-sanitize';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { AdminInfo } from '../../components/AdminInfo/AdminInfo';
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

export const WelcomeStep = () => {
  const { LL } = useI18nContext();
  const [sessionEnd, sessionStart] = useEnrollmentStore((state) => [
    state.sessionEnd,
    state.sessionStart,
  ]);
  const userInfo = useEnrollmentStore((state) => state.userInfo);

  const [nextSubject, next] = useEnrollmentStore(
    (state) => [state.nextSubject, state.nextStep],
    shallow,
  );

  const markdown = useMemo(() => {
    const startDay = dayjs(sessionStart);
    const endDay = dayjs(sessionEnd);
    const diffMils = endDay.diff(startDay);
    const mins = Math.ceil(diffMils / (1000 * 60));

    return LL.pages.enrollment.steps.welcome.explanation({
      time: mins.toString(),
    });
  }, [LL.pages.enrollment.steps.welcome, sessionEnd, sessionStart]);

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      next();
    });
    return () => {
      sub.unsubscribe();
    };
  }, [next, nextSubject]);

  return (
    <>
      <Card id="enrollment-welcome-card">
        <EnrollmentStepIndicator />
        <h3>
          {LL.pages.enrollment.steps.welcome.title({ name: `${userInfo?.first_name}` })}
        </h3>
        <div className="explenation">
          <ReactMarkdown rehypePlugins={[rehypeSanitaze]}>{markdown}</ReactMarkdown>
        </div>
        <AdminInfo />
      </Card>
    </>
  );
};
