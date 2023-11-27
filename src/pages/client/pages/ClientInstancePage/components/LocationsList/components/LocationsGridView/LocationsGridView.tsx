import './style.scss';

import { useQuery } from '@tanstack/react-query';
import classNames from 'classnames';

import { Card } from '../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
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

type Props = {
  instanceId: DefguardInstance['id'];
  locations: DefguardLocation[];
};

export const LocationsGridView = ({ instanceId, locations }: Props) => {
  return (
    <div id="locations-grid-view">
      {locations.map((l) => (
        <GridItem location={l} key={`${instanceId}${l.id}`} />
      ))}
    </div>
  );
};

type GridItemProps = {
  location: DefguardLocation;
};

const GridItem = ({ location }: GridItemProps) => {
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
    queryKey: [clientQueryKeys.getConnections, location.id as number],
    queryFn: () => getLastConnection({ locationId: location.id as number }),
    enabled: !!location.id,
  });
  const { data: locationStats } = useQuery({
    queryKey: [clientQueryKeys.getLocationStats, location.id as number, statsFilter],
    queryFn: () =>
      getLocationStats({
        locationId: location.id as number,
        from: getStatsFilterValue(statsFilter),
      }),
    enabled: !!location.id,
  });

  return (
    <Card className={cn}>
      <div className="top">
        <LocationCardTitle location={location} />
        <LocationCardConnectButton location={location} />
      </div>
      <div className="route">
        <LocationCardRoute location={location} />
      </div>
      {lastConnection && location && (
        <div className="info">
          <LocationCardInfo location={location} connection={lastConnection} />
        </div>
      )}
      {!lastConnection && <LocationCardNeverConnected />}
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
