import './style.scss';

import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import clsx from 'clsx';
import { Fragment, useMemo } from 'react';
import {
  ConnectModalView,
  mfaMethodToConnectModalView,
} from '../../../pages/full/OverviewPage/components/ConnectModal/hooks/types';
import { useConnectModal } from '../../../pages/full/OverviewPage/components/ConnectModal/hooks/useConnectModal';
import { useConfigureFactorsScreen } from '../../hooks/useConfigureFactorsScreen';
import { useConnectionAbility } from '../../hooks/useConnectionAbility';
import { api } from '../../rust-api/api';
import { getAppConfigQueryOptions } from '../../rust-api/query';
import type { InstanceInfo, LocationInfo } from '../../rust-api/types';
import { ThemeSpacing } from '../../types';
import { connectConfigureFactorsSource } from '../../utils/configureFactorsSource';
import { isPresent } from '../../utils/isPresent';
import { ConnectionAbility, resolveMfaStepPlan, shouldStartMfa } from '../../utils/mfa';
import { Divider } from '../Divider/Divider';
import { IconKind } from '../Icon';
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

  const { mutate: connect, isPending: isConnecting } = useMutation({
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
      } else if (connectError?.kind === 'allTrafficConflict') {
        useConnectModal.getState().open({
          location,
          view: ConnectModalView.ConnectionError,
          connectionError: connectError.message,
        });
      } else if (connectError?.kind === 'serviceUnavailable') {
        useConnectModal.getState().open({
          location,
          view: ConnectModalView.ConnectionError,
        });
      }
    },
    meta: {
      invalidate: ['locations'],
    },
  });

  const { mutate: disconnect, isPending: isDisconnecting } = useMutation({
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

  const { mutate: configureMfa, isPending: isOpeningConfiguration } =
    useConfigureFactorsScreen();

  const isBusy = isConnecting || isDisconnecting || isOpeningConfiguration;

  const connectionAbility = useConnectionAbility(location, instance);

  const handleConnectClick = () => {
    if (!appConfig) return;
    if (location.active) {
      disconnect({ connectionType: location.connection_type, locationId: location.id });
      return;
    }

    if (connectionAbility === ConnectionAbility.Configurable) {
      // Goes through the backend like the tray card does, so both open the same screen.
      configureMfa({
        instanceId: location.instance_id,
        methods: [],
        source: connectConfigureFactorsSource(),
        locationId: location.id,
      });
      return;
    }

    if (shouldStartMfa(location)) {
      const stepPlan = resolveMfaStepPlan(location);
      useConnectModal.getState().open({
        view: mfaMethodToConnectModalView(stepPlan[0]),
        location,
        autoStartOpenId: appConfig.auto_start_openid_mfa,
        mfaMethod: stepPlan[0],
      });
      return;
    }

    connect({ connectionType: location.connection_type, locationId: location.id });
  };

  const canConfigureMfa = connectionAbility === ConnectionAbility.Configurable;

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
          <ConnectButton
            icon={canConfigureMfa ? IconKind.ManageKeys : null}
            text={canConfigureMfa ? 'Configure MFA' : null}
            active={location.active}
            onClick={handleConnectClick}
            disabled={
              isBusy ||
              (!location.active && connectionAbility === ConnectionAbility.Unavailable)
            }
          />
        </div>
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <div className="controls">
        {location.active && (
          <LocationCardConnectionTiles
            location={location}
            instance={instance}
            variant="full"
          />
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
            {shouldStartMfa(location) && (
              <LocationCardMfaEdit
                variant="full"
                location={location}
                connectionAbility={connectionAbility}
                onEdit={() => {
                  if (isPresent(location)) {
                    useConnectModal.getState().open({
                      view: ConnectModalView.MfaSettings,
                      location: location,
                      perviousView: null,
                      mfaMethod: resolveMfaStepPlan(location)[0],
                    });
                  }
                }}
              />
            )}
          </Fragment>
        )}
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <LocationCardConnectionInfo location={location} />
    </div>
  );
};
