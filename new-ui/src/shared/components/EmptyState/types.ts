import type { Ref } from 'react';
import type { ButtonProps } from '../Button/types';

export type EmptyStateProps = {
  ref?: Ref<HTMLDivElement>;
  title?: string;
  subtitle?: string;
  icon?: EmptyIconValue;
  className?: string;
  testId?: string;
  id?: string;
  primaryAction?: ButtonProps;
  secondaryAction?: () => void;
  secondaryActionText?: string;
};

export const EmptyIcon = {
  SessionTimeout: 'session-timeout',
  ServiceUnavailable: 'service-unavailable',
} as const;

export type EmptyIconValue = (typeof EmptyIcon)[keyof typeof EmptyIcon];
