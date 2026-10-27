import './style.scss';
import clsx from 'clsx';
import { TooltipContent } from '../../providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../providers/tooltip/TooltipTrigger';
import { ThemeVariable } from '../../types';
import { isPresent } from '../../utils/isPresent';
import { Icon, IconKind } from '../Icon';
import type { ToggleProps } from './types';

export const Toggle = ({
  active,
  testId,
  label,
  helper,
  disabled = false,
  onClick,
}: ToggleProps) => {
  return (
    <div
      className={clsx('toggle', {
        disabled,
        active,
        'with-helper': isPresent(helper),
      })}
      role="button"
      aria-disabled={disabled}
      data-testid={testId}
      data-value={active}
      onClick={(e) => {
        if (!disabled) {
          onClick?.(e);
        }
      }}
    >
      <div className="inner">
        <div className="circle"></div>
      </div>
      {isPresent(label) && <p>{label}</p>}
      {isPresent(helper) && (
        <TooltipProvider placement="top">
          <TooltipTrigger>
            <div className="helper" onClick={(e) => e.stopPropagation()}>
              <Icon
                icon={IconKind.Help}
                size={20}
                staticColor={ThemeVariable.FgWhite60}
              />
            </div>
          </TooltipTrigger>
          <TooltipContent>
            <p>{helper}</p>
          </TooltipContent>
        </TooltipProvider>
      )}
    </div>
  );
};
