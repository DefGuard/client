import './style.scss';
import clsx from 'clsx';

interface Props {
  active: boolean;
  onClick: () => void;
  disabled?: boolean;
}

export const ConnectButton = ({ active, onClick, disabled = false }: Props) => (
  <button
    type="button"
    className={clsx('connect-button', {
      connected: active,
      disconnected: !active,
    })}
    disabled={disabled}
    onClick={onClick}
  >
    <p>{active ? 'Disconnect' : 'Connect VPN'}</p>
  </button>
);
