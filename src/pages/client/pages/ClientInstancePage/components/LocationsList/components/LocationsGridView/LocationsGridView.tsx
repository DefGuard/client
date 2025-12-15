import './style.scss';

import { useQuery } from '@tanstack/react-query';
import classNames from 'classnames';
import parser from 'html-react-parser';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { Label } from '../../../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { getStatsFilterValue } from '../../../../../../../../shared/utils/getStatsFilterValue';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../../../query';
import type {
  ClientConnectionType,
  CommonWireguardFields,
  DefguardInstance,
} from '../../../../../../types';
import { LocationUsageChart } from '../../../LocationUsageChart/LocationUsageChart';
import { LocationUsageChartType } from '../../../LocationUsageChart/types';
import { LocationCardConnectButton } from '../LocationCardConnectButton/LocationCardConnectButton';
import { LocationCardInfo } from '../LocationCardInfo/LocationCardInfo';
import { LocationCardNeverConnected } from '../LocationCardNeverConnected/LocationCardNeverConnected';
import { LocationCardNoStats } from '../LocationCardNoStats/LocationCardNoStats';
import { LocationCardRoute } from '../LocationCardRoute/LocationCardRoute';
import { LocationCardTitle } from '../LocationCardTitle/LocationCardTitle';

type Props = {
  locations: CommonWireguardFields[];
  selectedDefguardInstance?: DefguardInstance;
};

export const LocationsGridView = ({ locations, selectedDefguardInstance }: Props) => {
  return (
    <div id="locations-grid-view">
      {locations.map((l) => (
        <GridItem
          location={l}
          selectedDefguardInstance={selectedDefguardInstance}
          key={`${l.name}${l.id}`}
        />
      ))}
    </div>
  );
};

type GridItemProps = {
  location: CommonWireguardFields;
  selectedDefguardInstance?: DefguardInstance;
};

const GridItem = ({ location, selectedDefguardInstance }: GridItemProps) => {
  const { LL } = useI18nContext();
  const cn = classNames(
    'grid-item',
    {
      active: location.active,
    },
    'no-info',
  );
  const { getLocationStats, getLastConnection } = clientApi;

  const statsFilter = useClientStore((state) => state.statsFilter);

  const { data: lastConnection } = useQuery({
    queryKey: [
      clientQueryKeys.getConnections,
      location.id as number,
      location.connection_type,
    ],
    queryFn: () =>
      getLastConnection({
        locationId: location.id as number,
        connectionType: location.connection_type,
      }),
    enabled: !!location.id,
  });
  const { data: locationStats } = useQuery({
    queryKey: [
      clientQueryKeys.getLocationStats,
      location.id as number,
      statsFilter,
      location.connection_type,
    ],
    queryFn: () =>
      getLocationStats({
        locationId: location.id as number,
        connectionType: location.connection_type as ClientConnectionType,
        from: getStatsFilterValue(statsFilter),
      }),
    enabled: !!location.id,
    refetchOnMount: true,
    refetchOnWindowFocus: true,
    refetchInterval: 10 * 1000,
  });

  return (
    <Card className={cn}>
      <div className="top">
        <LocationCardTitle location={location} />
        <LocationCardConnectButton location={location} />
      </div>
      <div className="route">
        <div className="top">
          <Label>{LL.pages.client.pages.instancePage.controls.traffic.label()}</Label>
          <Helper
            initialPlacement="right"
            icon={
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 12 12"
                width={12}
                height={12}
                fill="none"
              >
                <path fill="#899CA8" d="M6.4 12a6 6 0 1 0 0-12 6 6 0 0 0 0 12Z" />
                <path
                  fill="#fff"
                  d="M7.067 5.333a.667.667 0 0 0-1.333 0v3.334a.667.667 0 0 0 1.333 0V5.333ZM7.067 3.333a.667.667 0 1 0-1.333 0 .667.667 0 0 0 1.333 0Z"
                />
              </svg>
            }
          >
            {parser(LL.pages.client.pages.instancePage.controls.traffic.helper())}
          </Helper>
        </div>
        <LocationCardRoute
          location={location}
          selectedDefguardInstance={selectedDefguardInstance}
        />
      </div>
      {lastConnection && location && (
        <div className="info">
          <LocationCardInfo location={location} connection={lastConnection} />
        </div>
      )}
      {!lastConnection && !location.active && <LocationCardNeverConnected />}
      {locationStats && locationStats.length > 0 && (
        <LocationUsageChart
          heightX={20}
          hideX={true}
          data={locationStats}
          type={LocationUsageChartType.BAR}
        />
      )}
      {(!locationStats || !locationStats.length) &&
        (lastConnection || location.active) && <LocationCardNoStats />}
    </Card>
  );
};
