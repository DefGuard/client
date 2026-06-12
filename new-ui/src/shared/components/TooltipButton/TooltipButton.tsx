import {
  autoUpdate,
  FloatingPortal,
  offset,
  shift,
  useFloating,
} from '@floating-ui/react';
import { Fragment, useEffect, useMemo, useState } from 'react';
import type { Subject } from 'rxjs';
import { Button } from '../Button/Button';
import type { ButtonProps } from '../Button/types';
import { Tooltip } from '../Tooltip/Tooltip';

interface Props {
  tooltipText: string;
  buttonProps: ButtonProps;
  tooltipTimeout?: number;
  tooltipTrigger?: Subject<void>;
}

export const TooltipButton = ({
  buttonProps,
  tooltipText,
  tooltipTrigger,
  tooltipTimeout = 1_500,
}: Props) => {
  const [tooltipVisible, setTooltipVisible] = useState(false);

  const { refs, floatingStyles } = useFloating({
    placement: 'top',
    whileElementsMounted: autoUpdate,
    middleware: [offset(15), shift({ padding: 4 })],
    open: tooltipVisible,
    onOpenChange: setTooltipVisible,
  });

  useEffect(() => {
    if (!tooltipTrigger) return;
    const sub = tooltipTrigger.subscribe(() => setTooltipVisible(true));
    return () => sub.unsubscribe();
  }, [tooltipTrigger]);

  useEffect(() => {
    if (!tooltipVisible) return;
    const timeout = setTimeout(() => setTooltipVisible(false), tooltipTimeout);
    return () => clearTimeout(timeout);
  }, [tooltipVisible, tooltipTimeout]);

  const referenceProps = useMemo((): ButtonProps => {
    const base: ButtonProps = { ...buttonProps, ref: refs.setReference };
    if (tooltipTrigger) return base;
    return {
      ...base,
      onClick: (e) => {
        buttonProps.onClick?.(e);
        setTooltipVisible(true);
      },
    };
  }, [buttonProps, refs.setReference, tooltipTrigger]);

  return (
    <Fragment>
      <Button {...referenceProps} />
      {tooltipVisible && (
        <FloatingPortal>
          <Tooltip style={floatingStyles} ref={refs.setFloating}>
            <p>{tooltipText}</p>
          </Tooltip>
        </FloatingPortal>
      )}
    </Fragment>
  );
};
