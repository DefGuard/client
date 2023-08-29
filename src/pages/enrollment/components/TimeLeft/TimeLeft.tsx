import './style.scss';

import dayjs from 'dayjs';
import { useCallback, useEffect, useState } from 'react';
import { timer } from 'rxjs';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

type Props = {
  disableLabel?: boolean;
};

export const TimeLeft = ({ disableLabel }: Props) => {
  const { LL } = useI18nContext();

  const sessionEnd = useEnrollmentStore((state) => state.sessionEnd);

  const [timeLeft, setTimeLeft] = useState('');

  const updateTimeLeft = useCallback(() => {
    if (sessionEnd) {
      const now = dayjs();
      const endDay = dayjs(sessionEnd);
      const diff = endDay.diff(now, 'seconds');
      const minutes = Math.floor(diff / 60);
      const seconds = diff % 60;
      let minutesString = '';
      if (minutes > 0) {
        if (minutes > 1) {
          minutesString = `${minutes} ${LL.time.minutes.prular()}`;
        } else {
          minutesString = `${minutes} ${LL.time.minutes.singular()}`;
        }
      }
      let secondsString = '';
      if (seconds > 0) {
        if (seconds > 1) {
          secondsString = `${seconds} ${LL.time.seconds.prular()}`;
        } else {
          secondsString = `${seconds} ${LL.time.seconds.singular()}`;
        }
      }
      setTimeLeft(`${minutesString} ${secondsString}`);
    }
  }, [sessionEnd, LL.time.seconds, LL.time.minutes]);

  useEffect(() => {
    if (sessionEnd) {
      const sub = timer(0, 1000).subscribe(() => updateTimeLeft());
      return () => {
        sub.unsubscribe();
      };
    }
  }, [sessionEnd, updateTimeLeft]);

  if (disableLabel) {
    return <span className="time-left solo">{timeLeft}</span>;
  }

  return (
    <div className="time-left">
      <p>
        {LL.pages.enrollment.timeLeft()}: <span>{timeLeft}</span>
      </p>
    </div>
  );
};
