import './style.scss';
import clsx from 'clsx';
import { isPresent } from '../../utils/isPresent';
import { Badge } from '../Badge/Badge';
import { CheckboxIndicator } from '../CheckboxIndicator/CheckboxIndicator';
import { Icon } from '../Icon';
import { RadioIndicator } from '../RadioIndicator/RadioIndicator';
import { type SelectionItemProps, SelectionItemVariant } from './types';

export const SelectionItem = ({
  text,
  icon,
  selected,
  badge,
  testId,
  onClick,
  onKeyDown,
  variant = SelectionItemVariant.Radio,
  disabled = false,
  className,
  ...props
}: SelectionItemProps) => {
  return (
    <div
      {...props}
      className={clsx('selection-item', `variant-${variant}`, className, {
        selected,
        disabled,
      })}
      role={variant}
      aria-checked={selected}
      aria-disabled={disabled}
      tabIndex={disabled ? -1 : 0}
      data-testid={testId}
      data-variant={variant}
      onClick={() => {
        if (!disabled) onClick?.();
      }}
      onKeyDown={(e) => {
        onKeyDown?.(e);
        if (disabled || (e.key !== 'Enter' && e.key !== ' ')) return;
        e.preventDefault();
        onClick?.();
      }}
    >
      <div className="track">
        <div className="side-col">
          <Icon icon={icon} size={20} />
        </div>
        <div className="content">
          <p className="text">{text}</p>
          {isPresent(badge) && <Badge text={badge} disabled={disabled} />}
        </div>
        <div className="side-col">
          {variant === SelectionItemVariant.Checkbox && (
            <CheckboxIndicator active={selected} disabled={disabled} size="sm" />
          )}
          {variant === SelectionItemVariant.Radio && (
            <RadioIndicator active={selected} disabled={disabled} />
          )}
        </div>
      </div>
    </div>
  );
};
