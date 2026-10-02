import { Checkbox } from '../../../../../../shared/components/Checkbox/Checkbox';
import { Icon } from '../../../../../../shared/components/Icon';
import { TooltipContent } from '../../../../../../shared/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../../../shared/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../../../shared/providers/tooltip/TooltipTrigger';
import type { MfaMethodValue } from '../../../../../../shared/rust-api/types';
import { ThemeVariable } from '../../../../../../shared/types';
import { isPresent } from '../../../../../../shared/utils/isPresent';
import { mfaToText } from '../../../../../../shared/utils/mfa';
import './style.scss';

/** What one row has to say, resolved by the step so the markup only reads it. */
export type MethodRowState = {
  method: MfaMethodValue;
  checked: boolean;
  /** Nothing left for the user to decide here, the hint or the badge says why. */
  disabled: boolean;
  /** The factor is already on the account, as opposed to one this flow will register. */
  configured: boolean;
  /** Why the row is disabled, when a badge does not already say it. */
  hint: string | null;
};

interface Props extends MethodRowState {
  onToggle: (method: MfaMethodValue) => void;
}

export const MethodRow = ({
  method,
  checked,
  disabled,
  configured,
  hint,
  onToggle,
}: Props) => (
  <div className="method-row">
    <Checkbox
      text={mfaToText(method)}
      active={checked}
      disabled={disabled}
      onClick={() => {
        onToggle(method);
      }}
    />
    {(configured || isPresent(hint)) && (
      <div className="right">
        {configured && (
          <div className="configured-badge">
            <p>{`Configured`}</p>
          </div>
        )}
        {isPresent(hint) && (
          <TooltipProvider>
            <TooltipTrigger>
              <Icon icon="question" size={20} staticColor={ThemeVariable.BgWhite50} />
            </TooltipTrigger>
            <TooltipContent>
              <p>{hint}</p>
            </TooltipContent>
          </TooltipProvider>
        )}
      </div>
    )}
  </div>
);
