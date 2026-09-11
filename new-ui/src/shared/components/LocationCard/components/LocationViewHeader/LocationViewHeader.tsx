import './style.scss';
import type { PropsWithChildren } from 'react';

interface Props extends PropsWithChildren {
  title: string;
}

export const LocationViewHeader = ({ title, children }: Props) => {
  return (
    <div className="location-card-view-header">
      <p className="title">{title}</p>
      {children}
    </div>
  );
};
