import './style.scss';
import { useMutation } from '@tanstack/react-query';
import clsx from 'clsx';
import { api } from '../../../../rust-api/api';
import { type LocationInfo, LocationMfaMode } from '../../../../rust-api/types';

export const ConnectButton = ({ location }: { location: LocationInfo }) => {
  const { mutate: connect } = useMutation({
    mutationFn: api.connect,
    meta: {
      invalidate: ['locations'],
    },
  });

  const { mutate: disconnect } = useMutation({
    mutationFn: api.disconnect,
    meta: {
      invalidate: ['locations'],
    },
  });

  if (location.location_mfa_mode !== LocationMfaMode.Disabled) return null;

  return (
    <button
      className={clsx('connect-button', {
        connected: location.active,
        disconnected: !location.active,
      })}
      onClick={() => {
        if (location.active) {
          disconnect({
            connectionType: location.connection_type,
            locationId: location.id,
          });
        } else {
          connect({
            connectionType: location.connection_type,
            locationId: location.id,
          });
        }
      }}
    >
      <p>{location.active ? 'Disconnect' : 'Connect VPN'}</p>
    </button>
  );
};
