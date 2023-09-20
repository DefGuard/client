import { useQuery } from '@tanstack/react-query';

import { clientApi } from '../../clientAPI/clientApi';
import { useClientStore } from '../../hooks/useClientStore';
import { clientQueryKeys } from '../../query';
import { LocationsGridView } from './components/LocationsGridView/LocationsGridView';

const { getLocations } = clientApi;

export const LocationsList = () => {
  const selectedInstance = useClientStore((state) => state.selectedInstance);

  const { data: locations } = useQuery({
    queryKey: [clientQueryKeys.getLocations, selectedInstance as number],
    queryFn: () => getLocations({ instanceId: selectedInstance as number }),
    enabled: !!selectedInstance,
  });

  if (!selectedInstance || !locations) return null;

  return (
    <>
      <LocationsGridView locations={locations} instanceId={selectedInstance} />
    </>
  );
};
