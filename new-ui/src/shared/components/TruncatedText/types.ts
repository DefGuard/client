import type { Placement } from '@floating-ui/react';

export const TruncatedTextTooltip = {
  WhenTruncated: 'when-truncated',
  Always: 'always',
} as const;

export type TruncatedTextTooltipValue =
  (typeof TruncatedTextTooltip)[keyof typeof TruncatedTextTooltip];

export type TruncatedTextProps = {
  text: string;
  tooltip?: TruncatedTextTooltipValue;
  tooltipPlacement?: Placement;
  delayTooltip?: boolean;
  className?: string;
  testId?: string;
};
