import { useMemo } from 'react';

import { useClientStore } from '../../hooks/useClientStore';
import { LocationsGridView } from './components/LocationsGridView/LocationsGridView';

export const LocationsList = () => {
  const instances = useClientStore((state) => state.instances);
  const selectedInstance = useClientStore((state) => state.selectedInstance);
  const locations = useMemo(() => {
    const selected = instances.find((i) => i.id === selectedInstance);
    if (selected) {
      return selected.locations;
    }
    return [];
  }, [selectedInstance, instances]);

  if (!selectedInstance) return null;

  return (
    <>
      <LocationsGridView locations={locations} instanceId={selectedInstance} />
    </>
  );
};
