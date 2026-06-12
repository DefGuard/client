import './style.scss';
import dayjs from 'dayjs';
import { useMemo } from 'react';
import { InfoBanner } from '../../../../../shared/components/InfoBanner/InfoBanner';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/types';
import { formatDuration } from '../../../../../shared/utils/formatDuration';
import { EnrollmentControls } from '../../components/EnrollmentControls/EnrollmentControls';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

export const WelcomeStep = () => {
  const enrollData = useEnrollmentStore((s) => s.startResponse);

  const timeLeft = useMemo(() => {
    const deadline = enrollData?.deadline_timestamp;
    if (!deadline) return '';
    const duration = dayjs.duration(dayjs.unix(deadline).diff(dayjs()));
    return formatDuration(duration);
  }, [enrollData?.deadline_timestamp]);

  if (!enrollData) return null;

  return (
    <div id="welcome-step" className="step-content">
      <header>
        <h1>Hello, {`${enrollData.user.first_name} ${enrollData.user.last_name}`}</h1>
        <SizedBox height={ThemeSpacing.Md} />
        <p>
          In order to gain access to the company infrastructure, we require you to
          complete this enrollment process. During this process, you will need to:
        </p>
      </header>
      <SizedBox height={ThemeSpacing.Xl} />
      <ul>
        <li>{`Create your password`}</li>
        <li>{`Configure MFA`}</li>
      </ul>
      <SizedBox height={ThemeSpacing.Xl2} />
      <InfoBanner
        message={`You have a time limit of ${timeLeft} to complete this process. If you have any questions, please consult your assigned admin. `}
      />
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="admin-info">
        <p>{enrollData.admin.name}</p>
        <a href={`mailto:${enrollData.admin.email}`}>{enrollData.admin.email}</a>
      </div>
      <EnrollmentControls
        onNext={() => {
          useEnrollmentStore.getState().next();
        }}
        disableBack
      />
    </div>
  );
};
