import type { HTMLAttributes, MouseEventHandler, Ref } from 'react';
import type { DirectionValue } from '../../types';
import type { IconKindValue } from '../Icon/icon-types';

export const ButtonVariant = {
  Primary: 'primary',
  Secondary: 'secondary',
  Critical: 'critical',
  Outlined: 'outlined',
} as const;

export type ButtonVariantValue = (typeof ButtonVariant)[keyof typeof ButtonVariant];

export const ButtonSize = {
  Primary: 'primary',
  Big: 'big',
} as const;
export type ButtonSizeValue = (typeof ButtonSize)[keyof typeof ButtonSize];

export type ButtonProps = {
  text: string;
  variant?: ButtonVariantValue;
  size?: ButtonSizeValue;
  iconLeft?: IconKindValue;
  iconRight?: IconKindValue;
  iconRightRotation?: DirectionValue;
  testId?: string;
  disabled?: boolean;
  loading?: boolean;
  containerProps?: Omit<HTMLAttributes<HTMLButtonElement>, 'onClick'>;
  onClick?: MouseEventHandler<HTMLButtonElement>;
  ref?: Ref<HTMLButtonElement>;
};
