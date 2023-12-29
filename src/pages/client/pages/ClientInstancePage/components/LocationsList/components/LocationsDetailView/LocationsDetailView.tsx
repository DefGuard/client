import './style.scss';

import { useMemo, useState } from 'react';

import { CardTabs } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/types';
import { CommonWireguardFields } from '../../../../../../types';
import { LocationConnectionHistory } from './components/LocationConnectionHistory/LocationConnectionHistory';
import { LocationDetailCard } from './components/LocationDetailCard/LocationDetailCard';
import { LocationDetails } from './components/LocationDetails/LocationDetails';

type Props = {
  locations: CommonWireguardFields[];
};

const findLocationById = (
  locations: CommonWireguardFields[],
  id: number,
): CommonWireguardFields | undefined => locations.find((location) => location.id === id);

export const LocationsDetailView = ({ locations }: Props) => {
  const [activeLocationId, setActiveLocationId] = useState<number>(locations[0].id);

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
    (): CommonWireguardFields | undefined =>
      findLocationById(locations, activeLocationId),
    [locations, activeLocationId],
  );

  return (
    <div id="locations-detail-view">
      <CardTabs tabs={tabs} />
      {activeLocation && <LocationDetailCard location={activeLocation} tabbed />}
      {activeLocation && (
        <LocationConnectionHistory
          locationId={activeLocation.id}
          connected={activeLocation.active}
        />
      )}
      {activeLocation && <LocationDetails locationId={activeLocation.id} />}
    </div>
  );
};
