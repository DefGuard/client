import clsx from 'clsx';
import './style.scss';
import { isPresent } from '../../utils/isPresent';
import { CheckboxIndicator } from '../CheckboxIndicator/CheckboxIndicator';
import { FieldError } from '../FieldError/FieldError';
import type { CheckboxProps } from './types';

export const Checkbox = ({
  text,
  error,
  testId,
  active = false,
  disabled = false,
  children,
  onClick,
}: CheckboxProps) => {
  const hasError = isPresent(error);

  return (
    <div
      className={clsx('checkbox', {
        disabled,
      })}
    >
      <div
        className={clsx('track', {
          text: isPresent(text),
          disabled: disabled,
          active: active,
          error: hasError,
        })}
        data-testid={testId}
        onClick={onClick}
        role="button"
        tabIndex={disabled ? -1 : 0}
        data-active={active}
      >
        <CheckboxIndicator disabled={disabled} error={hasError} active={active} />
        {isPresent(text) && <span>{text}</span>}
        {isPresent(children) && <div className="custom-label">{children}</div>}
      </div>
      {isPresent(error) && error.length > 0 && <FieldError error={error} />}
    </div>
  );
};
