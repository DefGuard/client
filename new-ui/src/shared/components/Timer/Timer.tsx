import './style.scss';
import dayjs from 'dayjs';
import { useEffect, useState } from 'react';
import { interval, type Subscription } from 'rxjs';
import { ThemeVariable } from '../../types';
import { formatDuration } from '../../utils/formatDuration';
import { Icon, IconKind } from '../Icon';
import type { TimerProps } from './types';

function formatTimeLeft(deadline: string): string | null {
  const diff = dayjs(deadline).diff(dayjs());
  if (diff <= 0) return null;
  return formatDuration(dayjs.duration(diff));
}

export const Timer = ({ deadline }: TimerProps) => {
  const [timeLeft, setTimeLeft] = useState<string | null>(() => formatTimeLeft(deadline));

  useEffect(() => {
    setTimeLeft(formatTimeLeft(deadline));

    const diff = dayjs(deadline).diff(dayjs.utc());
    if (diff <= 0) return;

    const sub: Subscription = interval(1_000).subscribe(() => {
      const label = formatTimeLeft(deadline);
      setTimeLeft(label);
      if (!label) sub.unsubscribe();
    });

    return () => sub.unsubscribe();
  }, [deadline]);

  if (!timeLeft) return null;

  return (
    <div className="timer">
      <Icon icon={IconKind.Transactions} staticColor={ThemeVariable.FgWhite60} />
      <p>{`Time left: ${timeLeft}`}</p>
    </div>
  );
};
