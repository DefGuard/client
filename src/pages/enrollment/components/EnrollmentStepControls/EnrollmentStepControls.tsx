import './style.scss';

import classNames from 'classnames';
import type { ReactNode } from 'react';

import { AdminInfo } from '../AdminInfo/AdminInfo';
import { TimeLeft } from '../TimeLeft/TimeLeft';

type Props = {
  children: ReactNode;
  className?: string;
};

export const EnrollmentStepControls = ({ children, className }: Props) => {
  const cn = classNames('controls', className);

  return (
    <div className={cn}>
      <div className="actions">{children}</div>
      <div className="mobile-info">
        <AdminInfo />
        <TimeLeft />
      </div>
    </div>
  );
};
