import './style.scss';
import clsx from 'clsx';
import { useTextOverflow } from '../../hooks/useTextOverflow';
import { TooltipContent } from '../../providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../providers/tooltip/TooltipTrigger';
import { type TruncatedTextProps, TruncatedTextTooltip } from './types';

const tooltipOpenDelayMs = 700;

export const TruncatedText = ({
  text,
  tooltip = TruncatedTextTooltip.WhenTruncated,
  tooltipPlacement = 'top',
  delayTooltip = false,
  className,
  testId,
}: TruncatedTextProps) => {
  const { ref, isOverflowing } = useTextOverflow();
  const detectOverflow = tooltip === TruncatedTextTooltip.WhenTruncated;

  return (
    <TooltipProvider
      placement={tooltipPlacement}
      openDelayMs={delayTooltip ? tooltipOpenDelayMs : 0}
      disabled={detectOverflow && !isOverflowing}
    >
      <TooltipTrigger ref={detectOverflow ? ref : undefined}>
        <span className={clsx('truncated-text', className)} data-testid={testId}>
          {text}
        </span>
      </TooltipTrigger>
      <TooltipContent>
        <p>{text}</p>
      </TooltipContent>
    </TooltipProvider>
  );
};
