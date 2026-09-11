import './style.scss';

import clsx from 'clsx';
import type { MouseEventHandler, Ref } from 'react';

type Props = {
  text: string;
  id?: string;
  ref?: Ref<HTMLDivElement>;
  required?: boolean;
  onClick?: MouseEventHandler<HTMLDivElement>;
};

export const FieldLabel = ({ text, ref, required, id, onClick }: Props) => {
  return (
    <div
      className={clsx('field-label', {
        required,
      })}
      ref={ref}
      id={id}
      onClick={onClick}
    >
      {required && (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="5"
          height="5"
          viewBox="0 0 5 5"
          fill="none"
          className="required-icon"
        >
          <path
            d="M1.332 4.128L0.6 3.684L1.368 2.52H0V1.608L1.368 1.62L0.6 0.456L1.332 0L2.064 1.224L2.808 0L3.528 0.456L2.76 1.62L4.128 1.608V2.52H2.76L3.528 3.684L2.808 4.128L2.064 2.916L1.332 4.128Z"
            fill="#FF494C"
          />
        </svg>
      )}
      <span>{text}</span>
    </div>
  );
};
