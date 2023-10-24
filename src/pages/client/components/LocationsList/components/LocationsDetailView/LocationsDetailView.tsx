import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { error } from 'tauri-plugin-log-api';
import { useBreakpoint } from 'use-breakpoint';

import { deviceBreakpoints } from '../../../../../../shared/constants';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { CardTabs } from '../../../../../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../../../../../shared/defguard-ui/components/Layout/CardTabs/types';
import { getStatsFilterValue } from '../../../../../../shared/utils/getStatsFilterValue';
import { clientApi } from '../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../query';
import { DefguardInstance, DefguardLocation } from '../../../../types';
import { LocationUsageChart } from '../../../LocationUsageChart/LocationUsageChart';
import { LocationCardConnectButton } from '../LocationCardConnectButton/LocationCardConnectButton';
import { LocationCardInfo } from '../LocationCardInfo/LocationCardInfo';
import { LocationCardNoData } from '../LocationCardNoData/LocationCardNoData';
import { LocationCardTitle } from '../LocationCardTitle/LocationCardTitle';
import { LocationConnectionHistory } from '../LocationConnectionHistory/LocationConnectionHistory';

type Props = {
  instanceId: DefguardInstance['id'];
  locations: DefguardLocation[];
};

const findLocationById = (
  locations: DefguardLocation[],
  id: number,
): DefguardLocation | undefined => locations.find((location) => location.id === id);

const { getLocationStats, getLastConnection, getConnectionHistory } = clientApi;

export const LocationsDetailView = ({ locations }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [activeLocationId, setActiveLocationId] = useState<number>(locations[0].id);
  const statsFilter = useClientStore((state) => state.statsFilter);

  const { data: locationStats } = useQuery({
    queryKey: [clientQueryKeys.getLocationStats, activeLocationId as number, statsFilter],
    queryFn: () =>
      getLocationStats({
        locationId: activeLocationId as number,
        from: getStatsFilterValue(statsFilter),
      }),
    enabled: !!activeLocationId,
    onError: (e) => {
      error(`Error retrieving location stats: ${e}`);
    },
  });

  const { data: connectionHistory } = useQuery({
    queryKey: [clientQueryKeys.getConnectionHistory, activeLocationId as number],
    queryFn: () => getConnectionHistory({ locationId: activeLocationId as number }),
    enabled: !!activeLocationId,
    onError: (e) => {
      error(`Error retrieving connection history: ${e}`);
    },
  });

  const { data: lastConnection } = useQuery({
    queryKey: [clientQueryKeys.getConnections, activeLocationId as number],
    queryFn: () => getLastConnection({ locationId: activeLocationId as number }),
    enabled: !!activeLocationId,
    onError: (e) => {
      error(`Error retrieving last connection: ${e}`);
    },
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

  return (
    <div id="locations-detail-view">
      <CardTabs tabs={tabs} />
      <Card className="detail-card">
        <div className="header">
          <LocationCardTitle location={findLocationById(locations, activeLocationId)} />
          {breakpoint === 'desktop' && (
            <LocationCardInfo
              location={findLocationById(locations, activeLocationId)}
              connection={lastConnection}
            />
          )}
          <LocationCardConnectButton
            location={findLocationById(locations, activeLocationId)}
          />
        </div>
        {breakpoint !== 'desktop' && (
          <div className="info">
            <LocationCardInfo
              location={findLocationById(locations, activeLocationId)}
              connection={lastConnection}
            />
          </div>
        )}
        {locationStats && locationStats.length ? (
          <LocationUsageChart barSize={4} data={locationStats} />
        ) : null}
        {connectionHistory && connectionHistory.length ? (
          <LocationConnectionHistory connections={connectionHistory} />
        ) : null}
        {(!locationStats || locationStats.length === 0) && <LocationCardNoData />}
      </Card>
    </div>
  );
};
