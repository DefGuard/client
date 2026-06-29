import './style.scss';
import clsx from 'clsx';
import { motion } from 'motion/react';
import { useMemo } from 'react';
import { Icon, IconKind } from '../../../components/Icon';
import { InteractionBox } from '../../../components/InteractionBox/InteractionBox';
import { LoaderSpinner } from '../../../components/LoaderSpinner/LoaderSpinner';
import { motionTransitionStandard } from '../../../consts';
import { isPresent } from '../../../utils/isPresent';
import { type SnackbarConfig, SnackbarVariant } from '../types';

const positioningPadding = 20;
const elementHeight = 36;

// hidden
const elementInitialPosition = elementHeight + 1;
// visible + shift for padding
const elementActivePosition = positioningPadding * -1;

type StyleVariant = 'default' | 'success' | 'critical';

export const SnackbarElement = ({
  data,
  onExitAnimationEnd,
  onClose,
}: {
  data: SnackbarConfig;
  onClose: () => void;
  onExitAnimationEnd: () => void;
}) => {
  const styleVariant = useMemo((): StyleVariant => {
    if (data.variant === SnackbarVariant.Error) return 'critical';
    if (data.variant === SnackbarVariant.Success) return 'success';
    return 'default';
  }, [data.variant]);

  const icon = useMemo(() => {
    if (data.icon) return data.icon;
    const variant = data.variant ?? SnackbarVariant.Default;
    if (variant === SnackbarVariant.Success) return IconKind.Check;
    if (variant === SnackbarVariant.Error) return IconKind.WarningFilled;
    if (variant === SnackbarVariant.Default) return IconKind.CheckFilled;
    return null;
  }, [data.icon, data.variant]);

  const canClick = !data.dismissible && data.variant !== SnackbarVariant.Loading;

  return (
    <motion.div
      transition={motionTransitionStandard}
      initial={{
        y: elementInitialPosition,
        opacity: 0,
      }}
      animate={{
        y: elementActivePosition,
        opacity: 1,
      }}
      exit={{
        y: elementInitialPosition,
        opacity: 0,
      }}
      onAnimationComplete={(target: { opacity: number }) => {
        if (target.opacity === 0) {
          onExitAnimationEnd();
        }
      }}
      className={clsx('snackbar', `variant-${styleVariant}`, {
        'can-click': canClick,
      })}
      style={{
        height: elementHeight,
      }}
      onClick={() => {
        if (canClick) {
          onClose();
        }
      }}
    >
      <div className="content-track">
        {isPresent(icon) && (
          <div className="snackbar-icon">
            <Icon icon={icon} size={20} />
          </div>
        )}
        {data.variant === SnackbarVariant.Loading && <LoaderSpinner size={20} />}
        {isPresent(data.customRender) && data.customRender()}
        {isPresent(data.text) && <p>{data.text}</p>}
        {isPresent(data.action) && (
          <button
            className="snackbar-action"
            id={data.action.actionId}
            onClick={data.action.onClick}
          >
            <span>{data.action.text}</span>
          </button>
        )}
        {isPresent(data.dismissible) && (
          <InteractionBox onClick={onClose} interactionSize={26}>
            <Icon icon="close" size={20} />
          </InteractionBox>
        )}
      </div>
    </motion.div>
  );
};
