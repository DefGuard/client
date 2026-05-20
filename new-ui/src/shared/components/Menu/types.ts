import type { HTMLAttributes, Ref } from 'react';
import type { IconKindValue } from '../Icon/icon-types';

export interface MenuProps extends HTMLAttributes<HTMLDivElement> {
  itemGroups: MenuItemsGroup[];
  ref?: Ref<HTMLDivElement>;
  testId?: string;
  onClose?: () => void;
}

export interface MenuItemsGroup {
  header?: MenuHeaderProps;
  items: MenuItemProps[];
}

export interface MenuItemProps {
  text: string;
  variant?: 'default' | 'danger';
  disabled?: boolean;
  icon?: IconKindValue;
  items?: MenuItemProps[];
  testId?: string;
  onClick?: () => void;
  onClose?: () => void;
}

export interface MenuHeaderProps {
  text: string;
  tooltip?: string;
  testId?: string;
  onClose?: () => void;
  onHelp?: () => void;
}
