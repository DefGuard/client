import type { PropsWithChildren } from 'react';

export interface CheckboxProps extends PropsWithChildren {
  testId?: string;
  active?: boolean;
  error?: string;
  disabled?: boolean;
  text?: string;
  onClick?: () => void;
}
