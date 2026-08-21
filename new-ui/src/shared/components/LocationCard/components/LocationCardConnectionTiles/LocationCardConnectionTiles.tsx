import './style.scss';
import clsx from 'clsx';
import { useMemo } from 'react';
import { useAppData } from '../../../../providers/AppDataContext';
import {
  ClientTrafficPolicy,
  type InstanceInfo,
  type LocationInfo,
  LocationMfaMode,
} from '../../../../rust-api/types';
import { isPresent } from '../../../../utils/isPresent';
import { mfaToText } from '../../../../utils/mfa';
import { BoxIcon } from '../../../BoxIcon/BoxIcon';
import { Icon, IconKind } from '../../../Icon';

interface Props {
  variant: 'compact' | 'full';
  location: LocationInfo;
  instance?: InstanceInfo;
}

export const LocationCardConnectionTiles = ({ location, instance, variant }: Props) => {
  const { connectionMfaMethod } = useAppData();

  const routeAllTraffic =
    location.route_all_traffic ||
    instance?.client_traffic_policy === ClientTrafficPolicy.ForceAllTraffic;

  const mfaMethod = useMemo(() => {
    const key = `${location.connection_type.toLowerCase()}-${location.id}`;
    const method = connectionMfaMethod[key];
    return method;
  }, [connectionMfaMethod, location.connection_type.toLowerCase, location.id]);

  return (
    <div className={clsx('location-card-connection-tiles', `variant-${variant}`)}>
      <div className="tile">
        <BoxIcon>
          <Icon icon={IconKind.Globe} />
        </BoxIcon>
        <p className="label">Allowed traffic</p>
        <p className="label-value">
          {routeAllTraffic ? 'All traffic' : 'Predefined traffic'}
        </p>
      </div>
      {location.location_mfa_mode !== LocationMfaMode.Disabled &&
        isPresent(mfaMethod) && (
          <div className="tile">
            <BoxIcon>
              <Icon icon={IconKind.LockClosed} />
            </BoxIcon>
            <p className="label">Active MFA</p>
            <p className="label-value">{mfaToText(mfaMethod)}</p>
          </div>
        )}
    </div>
  );
};
