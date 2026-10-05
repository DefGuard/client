import { TooltipContent } from '../../../../providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../providers/tooltip/TooltipTrigger';
import { isPresent } from '../../../../utils/isPresent';
import { Icon, type IconKindValue } from '../../../Icon';
import './style.scss';
import clsx from 'clsx';

interface Props {
  active: boolean;
  onClick: () => void;
  disabled?: boolean;
  icon?: IconKindValue | null;
  text?: string | null;
  tooltip?: string | null;
}

export const ConnectButton = ({
  active,
  onClick,
  icon,
  text,
  tooltip,
  disabled = false,
}: Props) => {
  const label = isPresent(text) ? text : active ? 'Disconnect' : 'Connect VPN';

  const button = (
    // aria-disabled instead of disabled, a disabled button gets no hover to open the tooltip
    <button
      type="button"
      className={clsx('connect-button', {
        connected: active,
        disconnected: !active,
        icon: isPresent(icon),
        disabled,
      })}
      aria-disabled={disabled}
      onClick={() => {
        if (!disabled) onClick();
      }}
    >
      {isPresent(icon) && <Icon icon={icon} size={20} />}
      <p>{label}</p>
    </button>
  );

  if (!isPresent(tooltip)) return button;

  return (
    <TooltipProvider placement="top">
      <TooltipTrigger>{button}</TooltipTrigger>
      <TooltipContent>
        <p>{tooltip}</p>
      </TooltipContent>
    </TooltipProvider>
  );
};
