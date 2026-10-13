import './style.scss';
import clsx from 'clsx';
import type { HTMLAttributes } from 'react';

type Props = {
  text: string;
  disabled?: boolean;
  testId?: string;
} & HTMLAttributes<HTMLDivElement>;

export const Badge = ({ text, testId, disabled = false, className, ...props }: Props) => {
  return (
    <div
      {...props}
      className={clsx('badge', className, { disabled })}
      data-testid={testId}
    >
      <span>{text}</span>
    </div>
  );
};
