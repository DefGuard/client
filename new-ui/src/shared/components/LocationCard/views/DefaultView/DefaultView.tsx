import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { api } from '../../../../rust-api/api';
import { LocationMfaMode, MfaMethod } from '../../../../rust-api/types';
import { ThemeSpacing } from '../../../../types';
import { mfaToText } from '../../../../utils/mfa';
import { Divider } from '../../../Divider/Divider';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { Toggle } from '../../../Toggle/Toggle';
import { ConnectButton } from '../../components/ConnectButton/ConnectButton';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const DefaultView = () => {
  const { location, setView } = useLocationCardContext();

  const mfaMethod = location.mfa_method ?? MfaMethod.Totp;

  const { mutate: updateRouting } = useMutation({
    mutationFn: api.updateLocationRouting,
    meta: {
      invalidate: ['locations'],
    },
  });

  return (
    <div className="location-view-default">
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
      {location.location_mfa_mode !== LocationMfaMode.Disabled && mfaMethod && (
        <div className="location-mfa-row">
          <div className="mfa-badge">
            <p>MFA</p>
          </div>
          <p className="name">{mfaToText(mfaMethod)}</p>
          <IconButton
            variant={IconButtonVariant.SmallSelected}
            icon="edit"
            onClick={() => {
              setView(LocationCardViews.MfaSettings);
            }}
          />
        </div>
      )}
      <SizedBox height={ThemeSpacing.Xl3} />
      <ConnectButton />
    </div>
  );
};
