import './style.scss';
import clsx from 'clsx';
import type { PropsWithChildren } from 'react';
import { WindowHeader } from '../../../shared/components/WindowHeader/WindowHeader';

interface Props extends PropsWithChildren {
  id?: string;
  className?: string;
}

export const FullPage = ({ id, className, children }: Props) => {
  return (
    <div className={clsx('full-page', className)} id={id}>
      <WindowHeader variant="desktop" />
      <div className="navigation">
        <p>Navigation placeholder</p>
      </div>
      <div className="page-content">
        <div className="scroll-container">{children}</div>
      </div>
    </div>
  );
};
