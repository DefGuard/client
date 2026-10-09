import { motion } from 'motion/react';
import { useEffect, useRef, useState } from 'react';
import { motionTransitionStandard } from '../../../../consts';
import { TooltipContent } from '../../../../providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../providers/tooltip/TooltipTrigger';
import { isPresent } from '../../../../utils/isPresent';
import { Icon, type IconKindValue } from '../../../Icon';
import { LoaderSpinner } from '../../../LoaderSpinner/LoaderSpinner';
import './style.scss';
import clsx from 'clsx';

interface Props {
  active: boolean;
  onClick: () => void;
  disabled?: boolean;
  loading?: boolean;
  icon?: IconKindValue | null;
  text?: string | null;
  tooltip?: string | null;
}

export const ConnectButton = ({
  active,
  onClick,
  icon,
  text,
  tooltip,
  disabled = false,
  loading = false,
}: Props) => {
  const label = isPresent(text) ? text : active ? 'Disconnect' : 'Connect VPN';
  const isLoading = loading && !disabled;
  const [swapDirection, setSwapDirection] = useState<'to-loading' | 'to-content' | null>(
    null,
  );
  const previousLoadingRef = useRef(isLoading);

  useEffect(() => {
    if (previousLoadingRef.current !== isLoading) {
      setSwapDirection(isLoading ? 'to-loading' : 'to-content');
      previousLoadingRef.current = isLoading;
    }
  }, [isLoading]);

  const contentTransition = {
    ...motionTransitionStandard,
    delay:
      !isLoading && swapDirection === 'to-content'
        ? motionTransitionStandard.duration
        : 0,
  };

  const loaderTransition = {
    ...motionTransitionStandard,
    delay:
      isLoading && swapDirection === 'to-loading' ? motionTransitionStandard.duration : 0,
  };

  const button = (
    // aria-disabled instead of disabled, a disabled button gets no hover to open the tooltip
    <button
      type="button"
      className={clsx('connect-button', {
        connected: active,
        disconnected: !active,
        icon: isPresent(icon),
        disabled,
        loading: isLoading,
      })}
      aria-disabled={disabled || loading}
      onClick={() => {
        if (!disabled && !loading) onClick();
      }}
    >
      <motion.div
        className="content"
        aria-hidden={isLoading}
        initial={false}
        animate={{ opacity: isLoading ? 0 : 1 }}
        transition={contentTransition}
      >
        {isPresent(icon) && <Icon icon={icon} size={20} />}
        <p>{label}</p>
      </motion.div>
      <motion.div
        className="loader-overlay"
        aria-hidden={!isLoading}
        initial={false}
        animate={{ opacity: isLoading ? 1 : 0 }}
        transition={loaderTransition}
      >
        <LoaderSpinner variant="primary" />
      </motion.div>
    </button>
  );

  if (!isPresent(tooltip)) return button;

  return (
    <TooltipProvider placement="top">
      <TooltipTrigger>{button}</TooltipTrigger>
      <TooltipContent>
        <p>{tooltip}</p>
      </TooltipContent>
    </TooltipProvider>
  );
};
