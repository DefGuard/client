import clsx from 'clsx';
import './style.scss';
import type { JSX, PropsWithChildren } from 'react';

interface Props extends PropsWithChildren {
  containerProps?: JSX.IntrinsicElements['main'];
}

export const CompactPage = ({ children, containerProps }: Props) => {
  return (
    <main {...containerProps} className={clsx('compact-page', containerProps?.className)}>
      {children}
    </main>
  );
};
