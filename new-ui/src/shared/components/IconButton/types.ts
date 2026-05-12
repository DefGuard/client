import type { MouseEventHandler, Ref } from 'react';
import type { DirectionValue } from '../../types';
import type { IconKindValue } from '../Icon/icon-types';

export const IconButtonVariant = {
  Big: 'big',
  BigSelected: 'big-selected',
  Small: 'small',
  SmallSelected: 'small-selected',
} as const;

export type IconButtonVariantValue =
  (typeof IconButtonVariant)[keyof typeof IconButtonVariant];

export type IconButtonProps = {
  variant: IconButtonVariantValue;
  icon: IconKindValue;
  iconRotation?: DirectionValue;
  ref?: Ref<HTMLDivElement>;
  className?: string;
  onClick?: MouseEventHandler<HTMLDivElement>;
};
