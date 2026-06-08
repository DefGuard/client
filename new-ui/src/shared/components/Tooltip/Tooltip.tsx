import './style.scss';
import clsx from 'clsx';
import { type MotionProps, motion } from 'motion/react';
import type { HTMLProps, PropsWithChildren, Ref } from 'react';
import { motionTransitionStandard } from '../../consts';

export const Tooltip = ({
  ref,
  children,
  className,
  ...rest
}: PropsWithChildren & {
  ref?: Ref<HTMLDivElement>;
} & HTMLProps<HTMLDivElement> &
  MotionProps) => {
  return (
    <motion.div
      className={clsx('tooltip', className)}
      ref={ref}
      transition={motionTransitionStandard}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      {...rest}
    >
      {children}
    </motion.div>
  );
};
