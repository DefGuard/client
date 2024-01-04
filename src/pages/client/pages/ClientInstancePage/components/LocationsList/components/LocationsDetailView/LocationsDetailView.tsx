import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { CardTabs } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/types';
import { routes } from '../../../../../../../../shared/routes';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../../../query';
import { CommonWireguardFields, WireguardInstanceType } from '../../../../../../types';
import { LocationConnectionHistory } from './components/LocationConnectionHistory/LocationConnectionHistory';
import { LocationDetailCard } from './components/LocationDetailCard/LocationDetailCard';
import { LocationDetails } from './components/LocationDetails/LocationDetails';

type Props = {
  locations: CommonWireguardFields[];
  connectionType?: WireguardInstanceType;
};

const findLocationById = (
  locations: CommonWireguardFields[],
  id: number,
): CommonWireguardFields | undefined => locations.find((location) => location.id === id);

const { getTunnels } = clientApi;

export const LocationsDetailView = ({
  locations,
  connectionType = WireguardInstanceType.DEFGUARD_INSTANCE,
}: Props) => {
  const [activeLocationId, setActiveLocationId] = useState<number | undefined>(
    locations[0]?.id ?? undefined,
  );

  const selectedInstance = useClientStore((state) => state.selectedInstance);

  const navigate = useNavigate();

  const tabs = useMemo(
    (): CardTabsData[] =>
      locations.map((location) => ({
        key: location.id,
        content: location.name,
        active: location.id === activeLocationId,
        onClick: () => setActiveLocationId(location.id),
      })),
    [locations, activeLocationId],
  );

  const activeLocation = useMemo((): CommonWireguardFields | undefined => {
    if (!isUndefined(activeLocationId)) {
      return findLocationById(locations, activeLocationId);
    }
    return undefined;
  }, [locations, activeLocationId]);

  useEffect(() => {
    if (activeLocationId === undefined) {
      navigate(routes.client.addInstance, { replace: true });
    }
  }, [activeLocationId, navigate]);

  const { data: tunnels } = useQuery({
    queryKey: [clientQueryKeys.getTunnels],
    queryFn: () => getTunnels(),
    enabled: !!(
      selectedInstance?.id && selectedInstance?.type === WireguardInstanceType.TUNNEL
    ),
  });

  const tunnel = tunnels?.find((tunnel) => tunnel.id === selectedInstance?.id);

  return (
    <div id="locations-detail-view">
      {connectionType === WireguardInstanceType.DEFGUARD_INSTANCE && (
        <>
          <CardTabs tabs={tabs} />
          {activeLocation && <LocationDetailCard location={activeLocation} tabbed />}
          {activeLocation && (
            <LocationConnectionHistory
              locationId={activeLocation.id}
              connected={activeLocation.active}
              connectionType={activeLocation.connection_type}
            />
          )}
          {activeLocation && (
            <LocationDetails
              locationId={activeLocation.id}
              connectionType={activeLocation.connection_type}
            />
          )}
        </>
      )}
      {connectionType === WireguardInstanceType.TUNNEL && (
        <>
          {tunnel && <LocationDetailCard location={tunnel} />}
          {tunnel && (
            <LocationConnectionHistory
              locationId={tunnel.id}
              connected={tunnel.active}
              connectionType={tunnel.connection_type}
            />
          )}
          {tunnel && (
            <LocationDetails
              locationId={tunnel.id}
              connectionType={tunnel.connection_type}
            />
          )}
        </>
      )}
    </div>
  );
};
