import './style.scss';
import clsx from 'clsx';
import { useMemo } from 'react';
import { useAppData } from '../../../../providers/AppDataContext';
import {
  ClientTrafficPolicy,
  type InstanceInfo,
  type LocationInfo,
} from '../../../../rust-api/types';
import { isPresent } from '../../../../utils/isPresent';
import { mfaStepsOf, mfaStepsToText, mfaToText } from '../../../../utils/mfa';
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

  const mfaLabel = useMemo(() => {
    const steps = mfaStepsOf(location);
    if (steps.length === 0) return 'Not required';
    if (steps.length > 1) return mfaStepsToText(steps.length);

    const { methods } = steps[0];
    if (methods.length === 1) return mfaToText(methods[0].method, instance);

    const key = `${location.connection_type.toLowerCase()}-${location.id}`;
    const pickedMethod = connectionMfaMethod[key];
    if (isPresent(pickedMethod)) return mfaToText(pickedMethod, instance);
    return mfaStepsToText(1);
  }, [connectionMfaMethod, location, instance]);

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
      <div className="tile">
        <BoxIcon>
          <Icon icon={IconKind.LockClosed} />
        </BoxIcon>
        <p className="label">Active MFA</p>
        <p className="label-value">{mfaLabel}</p>
      </div>
    </div>
  );
};
