import './style.scss';

import classNames from 'classnames';

import { useQuery } from '@tanstack/react-query';
import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { DefguardInstance, DefguardLocation, Connection } from '../../../../types';
import { LocationCardConnectButton } from '../LocationCardConnectButton/LocationCardConnectButton';
import { LocationCardTitle } from '../LocationCardTitle/LocationCardTitle';
import { LocationCardInfo } from '../LocationCardInfo/LocationCardInfo';
import { LocationUsageChart } from '../../../LocationUsageChart/LocationUsageChart';
import { clientApi } from '../../../../clientAPI/clientApi';
import { clientQueryKeys } from '../../../../query';

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
  const { LL } = useI18nContext();
  const cn = classNames(
    'grid-item',
    {
      active: location.active,
    },
    'no-info',
  );
const { getLocationStats, getLastConnection } = clientApi;

  const { data: connection } = useQuery({
    queryKey: [clientQueryKeys.getConnections, location.id as number],
    queryFn: () => getLastConnection({ locationId: location.id as number }),
    enabled: !!location.id,
  });
  const { data: locationStats } = useQuery({
    queryKey: [clientQueryKeys.getLocationStats, location.id as number],
    queryFn: () => getLocationStats({ locationId: location.id as number }),
    enabled: !!location.id,
  });

  console.log(connection);
  return (
    <Card className={cn}>
      <div className="top">
        <LocationCardTitle location={location} />
        <LocationCardConnectButton location={location} />
      </div>
      {connection ? (
        <>
        <div className="info">
          <LocationCardInfo location={location} connection={connection} />
        </div>
        {locationStats ? <LocationUsageChart height={22} data={locationStats} /> : null}
        </>
      ) : (
        <p className="no-data">{LL.pages.client.locationNoData()}</p>
      )}
      <div className="stats"></div>
    </Card>
  );
};
