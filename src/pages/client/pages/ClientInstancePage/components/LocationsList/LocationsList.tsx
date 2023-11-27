import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../query';
import { ClientView } from '../../../../types';
import { LocationsDetailView } from './components/LocationsDetailView/LocationsDetailView';
import { LocationsGridView } from './components/LocationsGridView/LocationsGridView';

const { getLocations } = clientApi;

export const LocationsList = () => {
  const { LL } = useI18nContext();
  const selectedInstance = useClientStore((state) => state.selectedInstance);

  const selectedView = useClientStore((state) => state.selectedView);

  const toaster = useToaster();

  const { data: locations, isError } = useQuery({
    queryKey: [clientQueryKeys.getLocations, selectedInstance as number],
    queryFn: () => getLocations({ instanceId: selectedInstance as number }),
    enabled: !!selectedInstance,
  });

  useEffect(() => {
    if (isError) {
      toaster.error(LL.common.messages.error());
    }
  }, [isError, toaster, LL.common.messages]);

  // TODO: add loader or another placeholder view pointing to opening enter token modal if no instances are found / present
  if (!selectedInstance || !locations) return null;

  return (
    <>
      {selectedView === ClientView.GRID && (
        <LocationsGridView locations={locations} instanceId={selectedInstance} />
      )}
      {selectedView === ClientView.DETAIL && (
        <LocationsDetailView locations={locations} instanceId={selectedInstance} />
      )}
    </>
  );
};
