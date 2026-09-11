import type { MouseEventHandler } from 'react';

export interface ToggleProps {
  active: boolean;
  disabled?: boolean;
  label?: string;
  onClick?: MouseEventHandler<HTMLDivElement>;
  testId?: string;
}
