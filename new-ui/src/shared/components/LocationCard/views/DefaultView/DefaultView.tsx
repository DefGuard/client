import { useMutation } from '@tanstack/react-query';
import { Fragment } from 'react/jsx-runtime';
import { api } from '../../../../rust-api/api';
import {
  ClientTrafficPolicy,
  LocationMfaMode,
  MfaMethod,
} from '../../../../rust-api/types';
import { ThemeSpacing } from '../../../../types';
import { Divider } from '../../../Divider/Divider';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { Toggle } from '../../../Toggle/Toggle';
import { LocationCardConnectButton } from '../../components/LocationCardConnectButton';
import { LocationCardMfaEdit } from '../../components/LocationCardMfaEdit/LocationCardMfaEdit';
import { useLocationCardContext } from '../../context/context';
import { LocationCardViews } from '../../context/types';

export const DefaultView = () => {
  const { location, instance, setView } = useLocationCardContext();

  const mfaMethod = location.mfa_method ?? MfaMethod.Totp;

  const { mutate: updateRouting } = useMutation({
    mutationFn: api.updateLocationRouting,
    meta: {
      invalidate: ['locations'],
    },
  });

  return (
    <div className="location-view-default">
      {instance.client_traffic_policy === ClientTrafficPolicy.None && (
        <Fragment>
          <Divider spacing={ThemeSpacing.Md} />
          <Toggle
            disabled={location.active}
            active={location.route_all_traffic}
            label={
              location.route_all_traffic
                ? 'All traffic is allowed'
                : 'Predefined traffic only'
            }
            onClick={() => {
              updateRouting({
                connectionType: location.connection_type,
                locationId: location.id,
                routeAllTraffic: !location.route_all_traffic,
              });
            }}
          />
        </Fragment>
      )}
      {location.location_mfa_mode !== LocationMfaMode.Disabled && mfaMethod && (
        <Fragment>
          <Divider spacing={ThemeSpacing.Md} />
          <LocationCardMfaEdit
            variant="compact"
            location={location}
            onEdit={() => {
              setView(LocationCardViews.MfaSettings);
            }}
          />
        </Fragment>
      )}
      <SizedBox height={ThemeSpacing.Xl3} />
      <LocationCardConnectButton />
    </div>
  );
};
