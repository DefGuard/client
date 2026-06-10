import clsx from 'clsx';
import type { PropsWithChildren } from 'react';

interface Props extends PropsWithChildren {
  id?: string;
  className?: string;
}

export const FullPage = ({ children, id, className }: Props) => {
  return (
    <div className={clsx('full-page page-content', className)} id={id}>
      <div className="scroll-container">{children}</div>
    </div>
  );
};
