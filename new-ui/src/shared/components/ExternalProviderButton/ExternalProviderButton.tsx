import './style.scss';
import { clsx } from 'clsx';
import { AnimatePresence, motion } from 'motion/react';
import { useMemo } from 'react';
import { motionTransitionStandard } from '../../consts';
import { LoaderSpinner } from '../LoaderSpinner/LoaderSpinner';
import customImage from './assets/custom.svg';
import googleImage from './assets/google.svg';
import jumpcloudImage from './assets/jumpcloud.svg';
import microsoftImage from './assets/microsoft.svg';
import oktaImage from './assets/okta.svg';
import type { ExternalProviderButtonProps } from './types';

export const ExternalProviderButton = ({
  text,
  testId,
  provider,
  onClick,
  ref,
  size = 'primary',
  variant = 'primary',
  type = 'button',
  disabled = false,
  loading = false,
  className,
  ...props
}: ExternalProviderButtonProps) => {
  const providerImage = useMemo(() => {
    switch (provider) {
      case 'microsoft':
        return microsoftImage;
      case 'google':
        return googleImage;
      case 'okta':
        return oktaImage;
      case 'jumpcloud':
        return jumpcloudImage;
      case 'custom':
        return customImage;
    }
  }, [provider]);

  return (
    <button
      {...props}
      data-variant={variant}
      data-testid={testId}
      ref={ref}
      type={type}
      disabled={disabled || loading}
      onClick={(e) => {
        if (!disabled && !loading) {
          onClick?.(e);
        }
      }}
      className={clsx(
        'external-provider-button',
        `variant-${variant}`,
        `size-${size}`,
        className,
        {
          disabled,
          loading: !disabled && loading,
        },
      )}
    >
      <img src={providerImage} width={16} height={16} loading="eager" />
      <span className="text">{text}</span>
      <AnimatePresence mode="wait">
        {loading && !disabled && (
          <motion.div
            className="loader-overlay"
            initial={{
              opacity: 0,
            }}
            animate={{
              opacity: 1,
            }}
            exit={{ opacity: 0 }}
            transition={motionTransitionStandard}
          >
            <LoaderSpinner />
          </motion.div>
        )}
      </AnimatePresence>
    </button>
  );
};
