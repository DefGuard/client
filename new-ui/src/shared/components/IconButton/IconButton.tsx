import './style.scss';
import clsx from 'clsx';
import { Icon } from '../Icon/Icon';
import { type IconButtonProps, IconButtonVariant } from './types';

export const IconButton = ({
  icon,
  ref,
  iconRotation,
  className,
  variant = IconButtonVariant.Big,
  onClick,
}: IconButtonProps) => {
  return (
    <div
      ref={ref}
      className={clsx('icon-button', className, `variant-${variant}`, {})}
      onClick={(e) => {
        onClick?.(e);
      }}
      role="button"
    >
      <Icon icon={icon} rotationDirection={iconRotation} />
    </div>
  );
};
