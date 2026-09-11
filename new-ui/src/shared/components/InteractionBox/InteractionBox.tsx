import { type MouseEventHandler, type PropsWithChildren, type Ref, useMemo } from 'react';
import './style.scss';
import clsx from 'clsx';

type Props = {
  interactionSize?: number;
  onClick?: MouseEventHandler<HTMLButtonElement>;
  id?: string;
  className?: string;
  ref?: Ref<HTMLDivElement>;
  disabled?: boolean;
  tabIndex?: number;
};

export const InteractionBox = ({
  onClick,
  className,
  id,
  ref,
  tabIndex,
  interactionSize,
  disabled = false,
  children,
}: Props & PropsWithChildren) => {
  const style = useMemo(() => {
    const res: Record<string, number | string> = {};
    if (interactionSize) {
      res['--interaction-size'] = `${interactionSize}px`;
    }
    return res;
  }, [interactionSize]);

  return (
    <div className={clsx('interaction-box', className)} ref={ref} id={id} style={style}>
      {children}
      <button
        type="button"
        onClick={onClick}
        disabled={disabled}
        tabIndex={tabIndex}
        onMouseDown={(e) => {
          e.preventDefault();
        }}
      ></button>
    </div>
  );
};
