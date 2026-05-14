import './style.scss';
import clsx from 'clsx';
import type { MouseEventHandler, Ref } from 'react';
import { ThemeVariable } from '../../types';

type Props = {
  active: boolean;
  disabled?: boolean;
  error?: boolean;
  onClick?: MouseEventHandler<HTMLDivElement>;
  ref?: Ref<HTMLDivElement>;
};

export const CheckboxIndicator = ({ error, active, disabled, ref, onClick }: Props) => {
  return (
    <div
      ref={ref}
      data-value={active}
      aria-disabled={disabled}
      onClick={onClick}
      className="checkbox-indicator"
    >
      <div className="box-positioner">
        <div
          className={clsx('box', {
            error,
            disabled,
            active,
          })}
        ></div>
        {active && (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="12"
            height="8"
            viewBox="0 0 12 8"
            fill="none"
          >
            <path
              fillRule="evenodd"
              clipRule="evenodd"
              d="M11.2137 1.47029L3.95843 8L0 4.43741L1.32326 2.96712L3.95843 5.33877L9.89039 0L11.2137 1.47029Z"
              fill={disabled ? ThemeVariable.FgWhite60 : ThemeVariable.FgAction}
            />
          </svg>
        )}
      </div>
    </div>
  );
};
