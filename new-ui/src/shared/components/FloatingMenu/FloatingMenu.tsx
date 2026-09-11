import './style.scss';
import clsx from 'clsx';
import type { HTMLProps, PropsWithChildren } from 'react';

interface Props extends PropsWithChildren {
  containerProps: HTMLProps<HTMLDivElement>;
}
export const FloatingMenu = ({ containerProps, children }: Props) => {
  return (
    <div {...containerProps} className={clsx(containerProps.className, 'floating-menu')}>
      {children}
    </div>
  );
};
