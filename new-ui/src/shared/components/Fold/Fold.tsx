import './style.scss';
import clsx from 'clsx';
import type { HTMLAttributes, PropsWithChildren, Ref } from 'react';

export const Fold = ({
  ref,
  className,
  children,
  open,
  contentClassName,
  ...rest
}: {
  open: boolean;
  ref?: Ref<HTMLDivElement>;
  contentClassName?: string;
} & PropsWithChildren &
  HTMLAttributes<HTMLDivElement>) => {
  return (
    <div
      ref={ref}
      {...rest}
      className={clsx('fold', className, {
        folded: !open,
      })}
    >
      <div className={clsx('fold-content', contentClassName)}>{children}</div>
    </div>
  );
};
