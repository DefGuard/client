import { FloatingPortal, useMergeRefs } from '@floating-ui/react';
import clsx from 'clsx';
import { AnimatePresence } from 'motion/react';
import { Tooltip } from '../../components/Tooltip/Tooltip';
import { useTooltipContext } from './TooltipContext';
import type { ToolTipContentProps } from './types';

type Props = ToolTipContentProps;

export const TooltipContent = ({
  style,
  ref: propRef,
  children,
  variant,
  ...props
}: Props) => {
  const context = useTooltipContext();
  const ref = useMergeRefs([context.refs.setFloating, propRef]);
  return (
    <AnimatePresence mode="wait">
      {context.open && (
        <FloatingPortal>
          <Tooltip
            ref={ref}
            style={{
              ...context.floatingStyles,
              ...style,
            }}
            {...context.getFloatingProps(props)}
            className={clsx(`variant-${variant}`, props.className)}
          >
            {children}
          </Tooltip>
        </FloatingPortal>
      )}
    </AnimatePresence>
  );
};
