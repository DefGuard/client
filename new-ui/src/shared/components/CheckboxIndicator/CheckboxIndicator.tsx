import './style.scss';
import clsx from 'clsx';
import type { MouseEventHandler, Ref } from 'react';

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
        {active && <CheckIcon />}
      </div>
    </div>
  );
};

const CheckIcon = () => {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M15.5 7.47029L8.38301 14L4.5 10.4374L5.79805 8.96712L8.38301 11.3388L14.2019 6L15.5 7.47029Z"
        fill="#7E8794"
      />
    </svg>
  );
};
