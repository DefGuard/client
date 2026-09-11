import './style.scss';
import type { PropsWithChildren } from 'react';

export const BoxIcon = ({ children }: PropsWithChildren) => {
  return <div className="box-icon">{children}</div>;
};
