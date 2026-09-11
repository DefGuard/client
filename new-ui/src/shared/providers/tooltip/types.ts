import type { Placement } from '@floating-ui/react';
import type { HTMLProps, Ref } from 'react';
import type { useTooltip } from './useTooltip';

export type TooltipContextType = ReturnType<typeof useTooltip> | null;

export interface TooltipOptions {
  disabled?: boolean;
  initialOpen?: boolean;
  placement?: Placement;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export type ToolTipContentProps = HTMLProps<HTMLDivElement> & {
  ref?: Ref<HTMLDivElement>;
  variant?: 'default' | 'light';
};
