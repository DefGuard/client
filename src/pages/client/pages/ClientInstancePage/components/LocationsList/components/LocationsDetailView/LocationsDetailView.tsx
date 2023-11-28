import './style.scss';

import { useQuery } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { deviceBreakpoints } from '../../../../../../../../shared/constants';
import { Card } from '../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { CardTabs } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/types';
import { Helper } from '../../../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { Label } from '../../../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { getStatsFilterValue } from '../../../../../../../../shared/utils/getStatsFilterValue';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../../../query';
import { DefguardInstance, DefguardLocation } from '../../../../../../types';
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
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage;
  const { breakpoint } = useBreakpoint({ ...deviceBreakpoints, desktop: 1300 });
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
    refetchOnWindowFocus: true,
    refetchOnMount: true,
  });

  const { data: connectionHistory } = useQuery({
    queryKey: [clientQueryKeys.getConnectionHistory, activeLocationId as number],
    queryFn: () => getConnectionHistory({ locationId: activeLocationId as number }),
    enabled: !!activeLocationId,
    refetchInterval: 10 * 1000,
    refetchOnWindowFocus: true,
    refetchOnMount: true,
  });

  const { data: lastConnection } = useQuery({
    queryKey: [clientQueryKeys.getConnections, activeLocationId as number],
    queryFn: () => getLastConnection({ locationId: activeLocationId as number }),
    enabled: !!activeLocationId,
    refetchInterval: 10 * 1000,
    refetchOnWindowFocus: true,
    refetchOnMount: true,
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
            <div className="route">
              {!activeLocation?.active && (
                <div className="controls">
                  <Helper initialPlacement="left">
                    {parse(localLL.controls.traffic.helper())}
                  </Helper>
                  <LocationCardRoute location={activeLocation} />
                </div>
              )}
              {activeLocation?.active && (
                <>
                  <Label>{localLL.controls.traffic.label()}</Label>
                  <p>
                    {activeLocation.route_all_traffic
                      ? localLL.controls.traffic.allTraffic()
                      : localLL.controls.traffic.predefinedTraffic()}
                  </p>
                </>
              )}
            </div>
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
        {breakpoint !== 'desktop' && (
          <div className="route">
            <div className="top">
              <Label>{localLL.controls.traffic.label()}</Label>
              <Helper
                initialPlacement="right"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width={12}
                    height={12}
                    fill="none"
                  >
                    <path
                      style={{
                        fill: 'var(--surface-icon-primary)',
                      }}
                      d="M6 12A6 6 0 1 0 6 0a6 6 0 0 0 0 12Z"
                    />
                    <path
                      style={{
                        fill: 'var(--surface-icon-secondary)',
                      }}
                      d="M6.667 5.333a.667.667 0 0 0-1.334 0v3.334a.667.667 0 0 0 1.334 0V5.333ZM6.667 3.333a.667.667 0 1 0-1.334 0 .667.667 0 0 0 1.334 0Z"
                    />
                  </svg>
                }
              >
                {parse(LL.pages.client.pages.instancePage.controls.traffic.helper())}
              </Helper>
            </div>
            <LocationCardRoute location={activeLocation} />
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
        {(!connectionHistory ||
          (connectionHistory.length === 0 && !activeLocation?.active)) && (
          <LocationCardNeverConnected />
        )}
      </Card>
    </div>
  );
};
