import './style.scss';
import type { PropsWithChildren } from 'react';

export const PlaygroundCard = ({ children }: PropsWithChildren) => {
  return <div className="playground-card">{children}</div>;
};
