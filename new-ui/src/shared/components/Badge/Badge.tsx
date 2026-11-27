import './style.scss';
import clsx from 'clsx';
import type { HTMLAttributes, Ref } from 'react';
import { isPresent } from '../../utils/isPresent';
import { Icon, type IconKindValue } from '../Icon';
import { BadgeVariant, type BadgeVariantValue } from './types';

type Props = {
  text: string;
  variant?: BadgeVariantValue;
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
  variant = BadgeVariant.Default,
  disabled = false,
  className,
  ...props
}: Props) => {
  return (
    <div
      {...props}
      ref={ref}
      className={clsx('badge', `variant-${variant}`, className, { disabled })}
      data-testid={testId}
      data-variant={variant}
    >
      {isPresent(icon) && <Icon icon={icon} size={16} />}
      <span>{text}</span>
    </div>
  );
};
