import './style.scss';
import type { PropsWithChildren } from 'react';
import { WindowHeader } from '../../components/WindowHeader/WindowHeader';
import { FullViewNavigation } from './components/FullViewNavigation/FullViewNavigation';

export const FullPageLayout = ({ children }: PropsWithChildren) => {
  return (
    <div className="full-page-layout">
      <WindowHeader variant="desktop" />
      <FullViewNavigation />
      {children}
    </div>
  );
};
