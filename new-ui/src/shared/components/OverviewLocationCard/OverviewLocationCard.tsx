import './style.scss';

import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import clsx from 'clsx';
import { Fragment, useMemo } from 'react';
import { ConnectModalView } from '../../../pages/full/OverviewPage/components/ConnectModal/hooks/types';
import { useConnectModal } from '../../../pages/full/OverviewPage/components/ConnectModal/hooks/useConnectModal';
import { api } from '../../rust-api/api';
import { getAppConfigQueryOptions } from '../../rust-api/query';
import type { InstanceInfo, LocationInfo } from '../../rust-api/types';
import { LocationMfaMode, MfaMethod } from '../../rust-api/types';
import { ThemeSpacing } from '../../types';
import { isPresent } from '../../utils/isPresent';
import { Divider } from '../Divider/Divider';
import { parseConnectError } from '../LocationCard/api/connectError';
import { ConnectButton } from '../LocationCard/components/ConnectButton/ConnectButton';
import { LocationCardConnectionInfo } from '../LocationCard/components/LocationCardConnectionInfo/LocationCardConnectionInfo';
import { LocationCardConnectionTiles } from '../LocationCard/components/LocationCardConnectionTiles/LocationCardConnectionTiles';
import { LocationCardHeaderInfo } from '../LocationCard/components/LocationCardHeaderInfo/LocationCardHeaderInfo';
import { LocationCardMfaEdit } from '../LocationCard/components/LocationCardMfaEdit/LocationCardMfaEdit';
import { Toggle } from '../Toggle/Toggle';

interface Props {
  location: LocationInfo;
  instance?: InstanceInfo;
}

export const OverviewLocationCard = ({ location, instance }: Props) => {
  const navigate = useNavigate();
  const { data: appConfig } = useQuery(getAppConfigQueryOptions);
  const { mutate: updateRouting } = useMutation({
    mutationFn: api.updateLocationRouting,
    meta: {
      invalidate: ['locations'],
    },
  });

  const { mutate: connect } = useMutation({
    mutationFn: api.connect,
    onError: (err) => {
      const connectError = parseConnectError(err);
      if (
        location.posture_check_required &&
        connectError?.kind === 'postureCheckFailed'
      ) {
        useConnectModal.getState().open({
          location,
          view: ConnectModalView.PostureCheckFail,
          postureError: connectError.message,
        });
      }
    },
    meta: {
      invalidate: ['locations'],
    },
  });

  const { mutate: disconnect } = useMutation({
    mutationFn: api.disconnect,
    meta: {
      invalidate: [
        ['locations'],
        ['active-connection'],
        ['connection-history'],
        ['alive-connections'],
      ],
    },
  });

  const handleConnectClick = () => {
    if (!appConfig) return;
    if (location.active) {
      disconnect({ connectionType: location.connection_type, locationId: location.id });
      return;
    }

    if (location.location_mfa_mode !== LocationMfaMode.Disabled) {
      const mfaMethod = location.mfa_method ?? MfaMethod.Totp;
      let view: (typeof ConnectModalView)[keyof typeof ConnectModalView];
      switch (mfaMethod) {
        case MfaMethod.Email:
          view = ConnectModalView.MfaEmail;
          break;
        case MfaMethod.Oidc:
          view = ConnectModalView.MfaOidc;
          break;
        case MfaMethod.MobileApprove:
          view = ConnectModalView.MfaMobile;
          break;
        default:
          view = ConnectModalView.MfaTotp;
      }
      useConnectModal.getState().open({
        view,
        location,
        autoStartOpenId: appConfig.auto_start_openid_mfa,
        mfaMethod: location.mfa_method,
      });
      return;
    }

    connect({ connectionType: location.connection_type, locationId: location.id });
  };

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
        <LocationCardHeaderInfo
          location={location}
          onInfoClick={() =>
            navigate({
              to: '/full/location-details',
              search: {
                locationId: location.id,
                locationName: location.name,
                connectionType: location.connection_type,
              },
            })
          }
        />
        <div className="right">
          <ConnectButton active={location.active} onClick={handleConnectClick} />
        </div>
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <div className="controls">
        {location.active && (
          <LocationCardConnectionTiles location={location} variant="full" />
        )}
        {!location.active && (
          <Fragment>
            {(instance?.client_traffic_policy === 'none' || !instance) && (
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
            )}
            <LocationCardMfaEdit
              variant="full"
              location={location}
              onEdit={() => {
                if (isPresent(location)) {
                  useConnectModal.getState().open({
                    view: ConnectModalView.MfaSettings,
                    location: location,
                    perviousView: null,
                    mfaMethod: location.mfa_method,
                  });
                }
              }}
            />
          </Fragment>
        )}
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <LocationCardConnectionInfo location={location} />
    </div>
  );
};
