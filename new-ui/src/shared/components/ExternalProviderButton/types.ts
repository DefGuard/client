import type { ButtonHTMLAttributes, HTMLAttributes, Ref } from 'react';

type DefaultButtonProps = ButtonHTMLAttributes<HTMLButtonElement>;

type ButtonVariant = 'primary' | 'secondary' | 'critical' | 'outlined';

type ButtonSize = 'primary' | 'big';

export type ExternalProviderButtonProps = {
  text: string;
  variant?: ButtonVariant;
  size?: ButtonSize;
  type?: DefaultButtonProps['type'];
  provider: 'microsoft' | 'google' | 'okta' | 'jumpcloud' | 'custom';
  testId?: string;
  disabled?: boolean;
  loading?: boolean;
  ref?: Ref<HTMLButtonElement>;
} & HTMLAttributes<HTMLElement>;
