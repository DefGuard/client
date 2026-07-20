import './style.scss';
import clsx from 'clsx';
import type { PropsWithChildren } from 'react';
import {
  Icon,
  IconKind,
  type IconKindValue,
} from '../../../../../../shared/components/Icon';
import { Direction, type DirectionValue } from '../../../../../../shared/types';

type Props = {
  onPrev: () => void;
  onNext: () => void;
  disablePrev: boolean;
  disableNext: boolean;
} & PropsWithChildren;

export const CarouselControls = ({
  onPrev,
  onNext,
  disablePrev,
  disableNext,
  children,
}: Props) => {
  return (
    <div className="carousel-controls">
      <ControlButton
        icon={IconKind.ArrowSmall}
        iconRotation={Direction.LEFT}
        onClick={onPrev}
        disabled={disablePrev}
      />
      {children}
      <ControlButton
        icon={IconKind.ArrowSmall}
        iconRotation={Direction.RIGHT}
        onClick={onNext}
        disabled={disableNext}
      />
    </div>
  );
};

const ControlButton = ({
  disabled,
  onClick,
  icon,
  iconRotation,
}: {
  disabled: boolean;
  onClick: () => void;
  icon: IconKindValue;
  iconRotation: DirectionValue;
}) => {
  return (
    <button
      className={clsx({
        disabled,
      })}
      onClick={onClick}
      disabled={disabled}
    >
      <Icon icon={icon} size={20} rotationDirection={iconRotation} />
    </button>
  );
};
