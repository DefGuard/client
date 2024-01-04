import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { routes } from '../../../../../../shared/routes';
import { clientApi } from '../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../query';
import { ClientView, WireguardInstanceType } from '../../../../types';
import { LocationsDetailView } from './components/LocationsDetailView/LocationsDetailView';
import { LocationsGridView } from './components/LocationsGridView/LocationsGridView';

const { getLocations, getTunnels } = clientApi;

export const LocationsList = () => {
  const { LL } = useI18nContext();
  const selectedInstance = useClientStore((state) => state.selectedInstance);

  const selectedView = useClientStore((state) => state.selectedView);

  const toaster = useToaster();

  const navigate = useNavigate();

  const queryKey = useMemo(() => {
    if (selectedInstance?.type === WireguardInstanceType.DEFGUARD_INSTANCE) {
      return [clientQueryKeys.getLocations, selectedInstance?.id as number];
    } else {
      return [clientQueryKeys.getTunnels];
    }
  }, [selectedInstance]);

  const queryFn = useCallback(() => {
    if (selectedInstance?.type === WireguardInstanceType.DEFGUARD_INSTANCE) {
      return getLocations({ instanceId: selectedInstance?.id as number });
    } else {
      return getTunnels();
    }
  }, [selectedInstance]);

  const { data: locations, isError } = useQuery({
    queryKey,
    queryFn,
    enabled: !!selectedInstance,
  });

  useEffect(() => {
    if (isError) {
      toaster.error(LL.common.messages.error());
    }
  }, [isError, toaster, LL.common.messages]);

  useEffect(() => {
    if (
      locations?.length === 0 &&
      selectedInstance?.type === WireguardInstanceType.TUNNEL
    ) {
      navigate(routes.client.addTunnel, { replace: true });
    }
  }, [locations, navigate, selectedInstance]);

  // TODO: add loader or another placeholder view pointing to opening enter token modal if no instances are found / present
  if (!selectedInstance || !locations) return null;

  return (
    <>
      {selectedView === ClientView.GRID &&
        selectedInstance.type === WireguardInstanceType.DEFGUARD_INSTANCE &&
        selectedInstance.id !== null && <LocationsGridView locations={locations} />}

      {selectedInstance.type === WireguardInstanceType.TUNNEL &&
        selectedInstance.id === undefined && <LocationsGridView locations={locations} />}

      {selectedView === ClientView.DETAIL &&
        selectedInstance.id &&
        selectedInstance.type === WireguardInstanceType.DEFGUARD_INSTANCE && (
          <LocationsDetailView
            locations={locations}
            connectionType={selectedInstance.type}
          />
        )}

      {selectedInstance.id && selectedInstance.type === WireguardInstanceType.TUNNEL && (
        <LocationsDetailView
          locations={locations}
          connectionType={selectedInstance.type}
        />
      )}
    </>
  );
};
