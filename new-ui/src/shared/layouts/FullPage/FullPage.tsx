import clsx from 'clsx';
import type { PropsWithChildren } from 'react';
import './style.scss';
import { ScrollContainer } from '../../components/ScrollContainer/ScrollContainer';

interface Props extends PropsWithChildren {
  id?: string;
  className?: string;
  hideScrollContainer?: boolean;
  /** Pins a trailing `Controls` row to the bottom of the page. */
  withControls?: boolean;
}

export const FullPage = ({
  children,
  id,
  className,
  hideScrollContainer = false,
  withControls = false,
}: Props) => {
  return (
    <div
      className={clsx('full-page page-content', className, {
        'with-controls': withControls,
        'scroll-hidden': hideScrollContainer,
      })}
      id={id}
    >
      {!hideScrollContainer && <ScrollContainer>{children}</ScrollContainer>}
      {hideScrollContainer && children}
    </div>
  );
};
