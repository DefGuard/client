import type { ReactNode } from 'react';
import type { IconKindValue } from '../../components/Icon';

export const SnackbarVariant = {
  Success: 'success',
  Warning: 'warning',
  Error: 'error',
  Loading: 'loading',
  Default: 'default',
} as const;

export type SnackbarVariantValue = (typeof SnackbarVariant)[keyof typeof SnackbarVariant];

export interface SnackbarAction {
  text: string;
  actionId?: string;
  onClick?: () => void;
}

export interface UpdateSnackbar {
  id: string;
  update: Partial<SnackbarConfig>;
  resetAutoDismiss?: boolean;
}

export interface SnackbarConfig {
  id?: string;
  icon?: IconKindValue;
  variant?: SnackbarVariantValue;
  text?: string;
  action?: SnackbarAction;
  dismissible?: boolean;
  customRender?: () => ReactNode;
}
