import './style.scss';
import clsx from 'clsx';

interface Props {
  active: boolean;
  onClick: () => void;
}

export const ConnectButton = ({ active, onClick }: Props) => (
  <button
    className={clsx('connect-button', {
      connected: active,
      disconnected: !active,
    })}
    onClick={onClick}
  >
    <p>{active ? 'Disconnect' : 'Connect VPN'}</p>
  </button>
);
