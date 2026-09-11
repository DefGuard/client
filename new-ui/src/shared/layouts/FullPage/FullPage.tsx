import clsx from 'clsx';
import type { PropsWithChildren } from 'react';

interface Props extends PropsWithChildren {
  id?: string;
  className?: string;
  hideScrollContainer?: boolean;
}

export const FullPage = ({
  children,
  id,
  className,
  hideScrollContainer = false,
}: Props) => {
  return (
    <div className={clsx('full-page page-content', className)} id={id}>
      {!hideScrollContainer && <div className="scroll-container">{children}</div>}
      {hideScrollContainer && children}
    </div>
  );
};
