import clsx from 'clsx';
import { type LocationInfo, LocationMfaMode } from '../../../../rust-api/types';
import { mfaToText } from '../../../../utils/mfa';
import { BoxIcon } from '../../../BoxIcon/BoxIcon';
import { Icon, IconKind } from '../../../Icon';

interface Props {
  variant: 'compact' | 'full';
  location: LocationInfo;
}

export const LocationCardConnectionTiles = ({ location, variant }: Props) => {
  return (
    <div className={clsx('location-card-connection-tiles', `variant-${variant}`)}>
      <div className="tile">
        <BoxIcon>
          <Icon icon={IconKind.Globe} />
        </BoxIcon>
        <p className="label">Allowed traffic</p>
        <p className="label-value">
          {location.route_all_traffic ? 'All traffic' : 'Predefined traffic'}
        </p>
      </div>
      {location.location_mfa_mode !== LocationMfaMode.Disabled && (
        <div className="tile">
          <BoxIcon>
            <Icon icon={IconKind.LockClosed} />
          </BoxIcon>
          <p className="label">Active MFA</p>
          <p className="label-value">{mfaToText(location.mfa_method ?? 'totp')}</p>
        </div>
      )}
    </div>
  );
};
