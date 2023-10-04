import { useQuery, useQueryClient } from '@tanstack/react-query';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';

import { clientApi } from '../../clientAPI/clientApi';
import { useClientStore } from '../../hooks/useClientStore';
import { clientQueryKeys } from '../../query';
import { LocationsDetailView } from './components/LocationsDetailView/LocationsDetailView';
import { LocationsGridView } from './components/LocationsGridView/LocationsGridView';

const { getLocations } = clientApi;

type Props = {
  layoutType: string;
};
export const LocationsList = ({ layoutType }: Props) => {
  const selectedInstance = useClientStore((state) => state.selectedInstance);

  const queryClient = useQueryClient();

  const { data: locations } = useQuery({
    queryKey: [clientQueryKeys.getLocations, selectedInstance as number],
    queryFn: () => getLocations({ instanceId: selectedInstance as number }),
    enabled: !!selectedInstance,
  });

  // listen to connection changes
  // TODO: move to main page component
  useEffect(() => {
    let cleanup: UnlistenFn | undefined;

    listen('connection-changed', () => {
      queryClient.invalidateQueries([clientQueryKeys.getLocations]);
      queryClient.invalidateQueries([clientQueryKeys.getConnections]);
      queryClient.invalidateQueries([clientQueryKeys.getActiveConnection]);
      queryClient.invalidateQueries([clientQueryKeys.getConnectionHistory]);
      queryClient.invalidateQueries([clientQueryKeys.getLocationStats]);
      queryClient.invalidateQueries([clientQueryKeys.getInstances]);
    }).then((c) => {
      cleanup = c;
    });

    return () => {
      cleanup?.();
    };
  }, [queryClient]);

  // TODO: add loader or another placeholder view pointing to opening enter token modal if no instances are found / present
  if (!selectedInstance || !locations) return null;

  return (
    <>
      {layoutType === 'GRID' ? (
        <LocationsGridView locations={locations} instanceId={selectedInstance} />
      ) : (
        <LocationsDetailView locations={locations} instanceId={selectedInstance} />
      )}
    </>
  );
};
