import type { HTMLAttributes } from 'react';
import type { IconKindValue } from '../Icon/icon-types';

export const SelectionItemVariant = {
  Radio: 'radio',
  Checkbox: 'checkbox',
} as const;

export type SelectionItemVariantValue =
  (typeof SelectionItemVariant)[keyof typeof SelectionItemVariant];

export type SelectionItemProps = {
  text: string;
  icon: IconKindValue;
  selected: boolean;
  badge?: string;
  variant?: SelectionItemVariantValue;
  disabled?: boolean;
  testId?: string;
  onClick?: () => void;
} & Omit<HTMLAttributes<HTMLDivElement>, 'onClick' | 'role'>;
