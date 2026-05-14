import './style.scss';
import { clsx } from 'clsx';
import { motion } from 'motion/react';
import { useEffect, useRef, useState } from 'react';
import { motionTransitionStandard } from '../../consts';
import { isPresent } from '../../utils/isPresent';
import { Icon } from '../Icon';
import { LoaderSpinner } from '../LoaderSpinner/LoaderSpinner';
import { type ButtonProps, ButtonSize, ButtonVariant } from './types';

export const Button = ({
  text,
  testId,
  iconLeft,
  iconRight,
  iconRightRotation,
  containerProps,
  onClick,
  size = ButtonSize.Primary,
  variant = ButtonVariant.Primary,
  disabled = false,
  loading = false,
}: ButtonProps) => {
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

  return (
    <div className={clsx("btn-wrap", `size-${size}`)}>
      <button
        {...containerProps}
        data-variant={variant}
        data-testid={testId}
        disabled={disabled || loading}
        onClick={(e) => {
          if (!disabled && !loading) {
            onClick?.(e);
          }
        }}
        className={clsx(
          'btn',
          `size-${size}`,
          `variant-${variant}`,
          containerProps?.className,
          {
            disabled,
            loading: isLoading,
            'icon-left': isPresent(iconLeft) && !isPresent(iconRight),
            'icon-right': isPresent(iconRight) && !isPresent(iconLeft),
            'icon-both': isPresent(iconLeft) && isPresent(iconRight),
          },
        )}
      >
        <motion.div
          className="btn-content"
          aria-hidden={isLoading}
          initial={false}
          animate={{ opacity: isLoading ? 0 : 1 }}
          transition={contentTransition}
        >
          {isPresent(iconLeft) && <Icon icon={iconLeft} size={20} />}
          <span className="text">{text}</span>
          {isPresent(iconRight) && (
            <Icon icon={iconRight} size={20} rotationDirection={iconRightRotation} />
          )}
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
    </div>
  );
};
