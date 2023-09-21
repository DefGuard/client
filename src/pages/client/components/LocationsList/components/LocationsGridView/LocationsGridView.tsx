import './style.scss';

import classNames from 'classnames';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { DefguardInstance, DefguardLocation } from '../../../../types';
import { LocationCardConnectButton } from '../LocationCardConnectButton/LocationCardConnectButton';
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
  const { LL } = useI18nContext();
  const cn = classNames(
    'grid-item',
    {
      active: location.active,
    },
    'no-info',
  );
  return (
    <Card className={cn}>
      <div className="top">
        <LocationCardTitle location={location} />
        <LocationCardConnectButton location={location} />
      </div>
      {/*
      <div className="info">
        <LocationCardInfo location={location} />
      </div>
      <div className="stats"></div>
      */}
      <p className="no-data">{LL.pages.client.locationNoData()}</p>
    </Card>
  );
};
