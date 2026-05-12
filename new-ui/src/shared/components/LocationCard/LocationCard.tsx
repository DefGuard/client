import './style.scss';
import { useMutation } from '@tanstack/react-query';
import clsx from 'clsx';
import { useState } from 'react';
import { api } from '../../rust-api/api';
import { type LocationInfo, LocationMfaMode } from '../../rust-api/types';
import { Direction, ThemeSpacing } from '../../types';
import { Divider } from '../Divider/Divider';
import { Fold } from '../Fold/Fold';
import { IconKind } from '../Icon';
import { IconButton } from '../IconButton/IconButton';
import { IconButtonVariant } from '../IconButton/types';
import { Toggle } from '../Toggle/Toggle';
import { LocationCardIcon } from './components/LocationCardIcon';

interface Props {
  location: LocationInfo;
}

export const LocationCard = ({ location }: Props) => {
  const [isOpen, setIsOpen] = useState(false);

  const { mutate: updateRouting } = useMutation({
    mutationFn: api.updateLocationRouting,
    meta: {
      invalidate: ['locations'],
    },
  });

  return (
    <div
      className={clsx('location-card')}
      data-network={location.network_id}
      data-id={location.id}
    >
      <div className="top-track">
        <div className="left">
          <LocationCardIcon />
          <div className="info">
            <p className="label">Location</p>
            <div className="bottom">
              <p className="location-name">{location.name}</p>
              {location.active && (
                <div className="online-badge">
                  <p>Online</p>
                </div>
              )}
            </div>
          </div>
        </div>
        <div className="right">
          <IconButton
            icon={IconKind.ArrowSmall}
            variant={isOpen ? IconButtonVariant.SmallSelected : IconButtonVariant.Small}
            iconRotation={isOpen ? Direction.DOWN : Direction.RIGHT}
            onClick={() => {
              setIsOpen((s) => !s);
            }}
          />
        </div>
      </div>
      <Fold open={isOpen}>
        <Divider spacing={ThemeSpacing.Md} />
        <Toggle
          disabled={location.active}
          active={location.route_all_traffic}
          label="All traffic is allowed"
          onClick={() => {
            updateRouting({
              connectionType: location.connection_type,
              locationId: location.id,
              routeAllTraffic: !location.route_all_traffic,
            });
          }}
        />
        <Divider spacing={ThemeSpacing.Md} />
        <ConnectButton location={location} />
      </Fold>
    </div>
  );
};

const ConnectButton = ({ location }: { location: LocationInfo }) => {
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
