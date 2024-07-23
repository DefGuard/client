import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { routes } from '../../../../../../shared/routes';
import { useClientStore } from '../../../../hooks/useClientStore';
import { CommonWireguardFields, WireguardInstanceType } from '../../../../types';
import { LocationsDetailView } from './components/LocationsDetailView/LocationsDetailView';
import { LocationsGridView } from './components/LocationsGridView/LocationsGridView';
import { MFAModal } from './modals/MFAModal/MFAModal';

interface LocationsListProps {
  locations: CommonWireguardFields[] | undefined;
  isError: boolean;
}

export const LocationsList = ({ locations, isError }: LocationsListProps) => {
  const { LL } = useI18nContext();

  const selectedView = useClientStore((state) => state.settings.selected_view);
  const selectedInstance = useClientStore((state) => state.selectedInstance);
  const toaster = useToaster();
  const navigate = useNavigate();

  const isTunnelType = selectedInstance?.type === WireguardInstanceType.TUNNEL;

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
      {locations.length === 1 && selectedView === null && !isTunnelType && (
        <LocationsDetailView
          locations={locations}
          connectionType={selectedInstance?.type}
        />
      )}
      {(selectedView === 'grid' || selectedView === null || isTunnelType) && (
        <LocationsGridView locations={locations} />
      )}

      {selectedView === 'detail' && !isTunnelType && (
        <LocationsDetailView
          locations={locations}
          connectionType={selectedInstance.type}
        />
      )}

      <MFAModal />
    </>
  );
};
