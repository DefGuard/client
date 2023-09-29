import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';

import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { CardTabs } from '../../../../../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../../../../../shared/defguard-ui/components/Layout/CardTabs/types';
import { clientApi } from '../../../../clientAPI/clientApi';
import { clientQueryKeys } from '../../../../query';
import { DefguardInstance, DefguardLocation } from '../../../../types';
import { LocationUsageChart } from '../../../LocationUsageChart/LocationUsageChart';
import { LocationCardConnectButton } from '../LocationCardConnectButton/LocationCardConnectButton';
import { LocationCardInfo } from '../LocationCardInfo/LocationCardInfo';
import { LocationCardTitle } from '../LocationCardTitle/LocationCardTitle';
import { LocationConnectionHistory } from '../LocationConnectionHistory/LocationConnectionHistory';

type Props = {
  instanceId: DefguardInstance['id'];
  locations: DefguardLocation[];
};

const { getLocationStats, getLastConnection } = clientApi;

export const LocationsDetailView = ({ instanceId, locations }: Props) => {
  const [activeLocationId, setActiveLocationId] = useState<number>(locations[0].id);

  const { data: locationStats } = useQuery({
    queryKey: [clientQueryKeys.getLocationStats, activeLocationId as number],
    queryFn: () => getLocationStats({ locationId: activeLocationId as number }),
    enabled: !!activeLocationId,
  });
  const { data: connection } = useQuery({
    queryKey: [clientQueryKeys.getConnections, activeLocationId as number],
    queryFn: () => getLastConnection({ locationId: activeLocationId as number }),
    enabled: !!activeLocationId,
  });

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

  const findLocationById = (
    locations: DefguardLocation[],
    id: number,
  ): DefguardLocation | undefined => locations.find((location) => location.id === id);

  console.log(locationStats);
  console.log(connection);

  return (
    <div id="locations-detail-view">
      <CardTabs tabs={tabs} />
      <Card className="detail-card" hideMobile shaded>
        <div className="header">
          <LocationCardTitle location={findLocationById(locations, activeLocationId)} />
          <LocationCardInfo
            location={findLocationById(locations, activeLocationId)}
            connection={connection}
          />
          <LocationCardConnectButton
            location={findLocationById(locations, activeLocationId)}
          />
        </div>
        {locationStats ? <LocationUsageChart height={200} data={locationStats} /> : null}
        <LocationConnectionHistory />
      </Card>
    </div>
  );
};
