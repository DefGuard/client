import type { HTMLProps, PropsWithChildren } from 'react';
import './style.scss';
import clsx from 'clsx';

type Props = PropsWithChildren & HTMLProps<HTMLDivElement>;

export const Controls = ({ children, className, ...props }: Props) => {
  return (
    <div {...props} className={clsx('controls', className)}>
      {children}
    </div>
  );
};
