import './style.scss';
import clsx from 'clsx';
import type { HTMLAttributes, Ref } from 'react';
import { isPresent } from '../../utils/isPresent';
import { Icon, type IconKindValue } from '../Icon';

type Props = {
  text: string;
  icon?: IconKindValue;
  disabled?: boolean;
  testId?: string;
  ref?: Ref<HTMLDivElement>;
} & HTMLAttributes<HTMLDivElement>;

export const Badge = ({
  text,
  icon,
  testId,
  ref,
  disabled = false,
  className,
  ...props
}: Props) => {
  return (
    <div
      {...props}
      ref={ref}
      className={clsx('badge', className, { disabled })}
      data-testid={testId}
    >
      {isPresent(icon) && <Icon icon={icon} size={16} />}
      <span>{text}</span>
    </div>
  );
};
