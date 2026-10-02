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
}

export const ConnectButton = ({
  active,
  onClick,
  icon,
  text,
  disabled = false,
}: Props) => {
  const label = isPresent(text) ? text : active ? 'Disconnect' : 'Connect VPN';

  return (
    <button
      type="button"
      className={clsx('connect-button', {
        connected: active,
        disconnected: !active,
        icon: isPresent(icon),
      })}
      disabled={disabled}
      onClick={onClick}
    >
      {isPresent(icon) && <Icon icon={icon} size={20} />}
      <p>{label}</p>
    </button>
  );
};
