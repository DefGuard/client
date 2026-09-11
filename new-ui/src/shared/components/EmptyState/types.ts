import type { Ref } from 'react';
import type { ButtonProps } from '../Button/types';

export type EmptyStateProps = {
  ref?: Ref<HTMLDivElement>;
  title?: string;
  subtitle?: string;
  icon?: string;
  className?: string;
  testId?: string;
  id?: string;
  primaryAction?: ButtonProps;
  secondaryAction?: () => void;
  secondaryActionText?: string;
};
