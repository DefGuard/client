import './style.scss';

import { isUndefined } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { CardTabs } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../../../../../../../shared/defguard-ui/components/Layout/CardTabs/types';
import { routes } from '../../../../../../../../shared/routes';
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
  const [activeLocationId, setActiveLocationId] = useState<number | undefined>(
    locations[0]?.id ?? undefined,
  );
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

  return (
    <div id="locations-detail-view">
      <CardTabs tabs={tabs} />
      {activeLocation && <LocationDetailCard location={activeLocation} tabbed />}
      {activeLocation && (
        <LocationConnectionHistory
          locationId={activeLocation.id}
          connected={activeLocation.active}
          locationType={activeLocation.location_type}
        />
      )}
      {activeLocation && (
        <LocationDetails
          locationId={activeLocation.id}
          locationType={activeLocation.location_type}
        />
      )}
    </div>
  );
};
