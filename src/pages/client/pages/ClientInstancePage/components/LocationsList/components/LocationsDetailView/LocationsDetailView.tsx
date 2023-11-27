import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { deviceBreakpoints } from '../../../../../../../../shared/constants';
import { Card } from '../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { CardTabs } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/types';
import { getStatsFilterValue } from '../../../../../../../../shared/utils/getStatsFilterValue';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../../../query';
import { DefguardInstance, DefguardLocation, RouteOption } from '../../../../../../types';
import { LocationUsageChart } from '../../../LocationUsageChart/LocationUsageChart';
import { LocationUsageChartType } from '../../../LocationUsageChart/types';
import { LocationCardConnectButton } from '../LocationCardConnectButton/LocationCardConnectButton';
import { LocationCardInfo } from '../LocationCardInfo/LocationCardInfo';
import { LocationCardNeverConnected } from '../LocationCardNeverConnected/LocationCardNeverConnected';
import { LocationCardNoStats } from '../LocationCardNoStats/LocationCardNoStats';
import { LocationCardRoute } from '../LocationCardRoute/LocationCardRoute';
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
    refetchInterval: 10 * 1000,
  });

  const { data: connectionHistory } = useQuery({
    queryKey: [clientQueryKeys.getConnectionHistory, activeLocationId as number],
    queryFn: () => getConnectionHistory({ locationId: activeLocationId as number }),
    enabled: !!activeLocationId,
    refetchInterval: 10 * 1000,
  });

  const { data: lastConnection } = useQuery({
    queryKey: [clientQueryKeys.getConnections, activeLocationId as number],
    queryFn: () => getLastConnection({ locationId: activeLocationId as number }),
    enabled: !!activeLocationId,
    refetchInterval: 10 * 1000,
  });
  const [routeOption, setRouteOption] = useState(RouteOption.PREDEFINED_TRAFFIC);

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

  const activeLocation = useMemo(
    (): DefguardLocation | undefined => findLocationById(locations, activeLocationId),
    [locations, activeLocationId],
  );

  return (
    <div id="locations-detail-view">
      <CardTabs tabs={tabs} />
      <Card className="detail-card">
        <div className="header">
          <LocationCardTitle location={activeLocation} />
          {breakpoint === 'desktop' && (
            <LocationCardInfo location={activeLocation} connection={lastConnection} />
          )}
          {breakpoint === 'desktop' && (
            <LocationCardRoute
              location={activeLocation}
              selected={routeOption}
              onChange={(v) => setRouteOption(v)}
            />
          )}
          <LocationCardConnectButton location={activeLocation} />
        </div>
        {breakpoint !== 'desktop' && (
          <div className="info">
            <LocationCardInfo
              location={findLocationById(locations, activeLocationId)}
              connection={lastConnection}
            />
          </div>
        )}
        {locationStats && locationStats.length > 0 && (
          <LocationUsageChart
            data={locationStats}
            type={LocationUsageChartType.LINE}
            margin={{ left: 20, right: 20 }}
          />
        )}
        {(!locationStats || locationStats.length == 0) &&
          ((connectionHistory && connectionHistory.length) || activeLocation?.active) && (
            <LocationCardNoStats />
          )}
        {connectionHistory && connectionHistory.length ? (
          <>
            <h2>Connection History</h2>
            <LocationConnectionHistory connections={connectionHistory} />
          </>
        ) : null}
        {(!connectionHistory || connectionHistory.length === 0) && (
          <LocationCardNeverConnected />
        )}
      </Card>
    </div>
  );
};
