import './style.scss';

import { useMutation } from '@tanstack/react-query';
import clsx from 'clsx';
import { useMemo } from 'react';
import { api } from '../../rust-api/api';
import type { LocationInfo } from '../../rust-api/types';
import { ThemeSpacing } from '../../types';
import { Divider } from '../Divider/Divider';
import { ConnectButton } from '../LocationCard/components/ConnectButton/ConnectButton';
import { LocationCardConnectionInfo } from '../LocationCard/components/LocationCardConnectionInfo/LocationCardConnectionInfo';
import { LocationCardHeaderInfo } from '../LocationCard/components/LocationCardHeaderInfo/LocationCardHeaderInfo';
import { LocationCardMfaEdit } from '../LocationCard/components/LocationCardMfaEdit/LocationCardMfaEdit';
import { Toggle } from '../Toggle/Toggle';

interface Props {
  location: LocationInfo;
}

export const OverviewLocationCard = ({ location }: Props) => {
  const { mutate: updateRouting } = useMutation({
    mutationFn: api.updateLocationRouting,
    meta: {
      invalidate: ['locations'],
    },
  });

  const traficLabel = useMemo(() => {
    if (location.route_all_traffic) {
      return 'All traffic is allowed';
    } else {
      return 'Predefined traffic only';
    }
  }, [location.route_all_traffic]);

  return (
    <div className={clsx('overview-location-card')}>
      <div className="header">
        <LocationCardHeaderInfo location={location} />
        <div className="right">
          <ConnectButton active={false} onClick={() => {}} />
        </div>
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <div className="controls">
        <Toggle
          disabled={location.active}
          active={location.route_all_traffic}
          label={traficLabel}
          onClick={() => {
            updateRouting({
              connectionType: location.connection_type,
              locationId: location.id,
              routeAllTraffic: !location.route_all_traffic,
            });
          }}
        />
        <LocationCardMfaEdit
          variant="full"
          location={location}
          onEdit={() => {
            console.log('Edit');
          }}
        />
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <LocationCardConnectionInfo location={location} />
    </div>
  );
};
