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
  disabled = false,
  onClick,
}: IconButtonProps) => {
  return (
    <div
      ref={ref}
      className={clsx('icon-button', className, `variant-${variant}`, { disabled })}
      onClick={(e) => {
        if (!disabled) {
          onClick?.(e);
        }
      }}
      role="button"
      aria-disabled={disabled}
    >
      <Icon icon={icon} rotationDirection={iconRotation} />
    </div>
  );
};
