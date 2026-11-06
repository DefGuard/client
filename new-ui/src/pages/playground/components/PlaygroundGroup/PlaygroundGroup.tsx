import './style.scss';
import type { PropsWithChildren } from 'react';

type Props = PropsWithChildren<{
  title: string;
}>;

export const PlaygroundGroup = ({ title, children }: Props) => {
  return (
    <div className="playground-group">
      <p className="title">{title}</p>
      {children}
    </div>
  );
};
