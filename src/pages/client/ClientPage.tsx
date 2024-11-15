import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../i18n/i18n-react';
import { useToaster } from '../../shared/defguard-ui/hooks/toasts/useToaster';
import { routes } from '../../shared/routes';
import { clientApi } from './clientAPI/clientApi';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
import { DeadConDroppedModal } from './components/modals/DeadConDroppedModal/DeadConDroppedModal';
import { useDeadConDroppedModal } from './components/modals/DeadConDroppedModal/store';
import { useClientFlags } from './hooks/useClientFlags';
import { useClientStore } from './hooks/useClientStore';
import { clientQueryKeys } from './query';
import { DeadConDroppedPayload, TauriEventKey } from './types';

const { getInstances, getTunnels } = clientApi;

export const ClientPage = () => {
  const queryClient = useQueryClient();
  const [setInstances, setTunnels] = useClientStore((state) => [
    state.setInstances,
    state.setTunnels,
  ]);
  const navigate = useNavigate();
  const firstLaunch = useClientFlags((state) => state.firstStart);
  const [listChecked, setListChecked] = useClientStore((state) => [
    state.listChecked,
    state.setListChecked,
  ]);
  const location = useLocation();
  const toaster = useToaster();
  const openDeadConDroppedModal = useDeadConDroppedModal((s) => s.open);
  const { LL } = useI18nContext();

  const { data: instances } = useQuery({
    queryFn: getInstances,
    queryKey: [clientQueryKeys.getInstances],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });
  const { data: tunnels } = useQuery({
    queryFn: getTunnels,
    queryKey: [clientQueryKeys.getTunnels],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    const instanceUpdate = listen(TauriEventKey.INSTANCE_UPDATE, () => {
      const invalidate = [
        clientQueryKeys.getInstances,
        clientQueryKeys.getLocations,
        clientQueryKeys.getTunnels,
      ];
      invalidate.forEach((key) =>
        queryClient.invalidateQueries({
          queryKey: [key],
        }),
      );
    });

    const locationUpdate = listen(TauriEventKey.LOCATION_UPDATE, () => {
      const invalidate = [clientQueryKeys.getLocations, clientQueryKeys.getTunnels];
      invalidate.forEach((key) =>
        queryClient.invalidateQueries({
          queryKey: [key],
        }),
      );
    });

    const connectionChanged = listen(TauriEventKey.CONNECTION_CHANGED, () => {
      const invalidate = [
        clientQueryKeys.getLocations,
        clientQueryKeys.getConnections,
        clientQueryKeys.getActiveConnection,
        clientQueryKeys.getConnectionHistory,
        clientQueryKeys.getLocationStats,
        clientQueryKeys.getInstances,
        clientQueryKeys.getTunnels,
      ];
      invalidate.forEach((key) =>
        queryClient.invalidateQueries({
          queryKey: [key],
        }),
      );
    });

    const configChanged = listen(TauriEventKey.CONFIG_CHANGED, (data) => {
      const instance = data.payload as string;
      toaster.info(LL.common.messages.configChanged({ instance }));
    });

    const deadConnectionDropped = listen<DeadConDroppedPayload>(
      TauriEventKey.DEAD_CONNECTION_DROPPED,
      (data) => {
        toaster.warning(
          LL.common.messages.deadConDropped({
            interface_name: data.payload.interface_name,
            con_type: data.payload.con_type,
          }),
          {
            lifetime: -1,
          },
        );
        openDeadConDroppedModal(data.payload);
      },
    );

    return () => {
      deadConnectionDropped.then((cleanup) => cleanup());
      configChanged.then((cleanup) => cleanup());
      connectionChanged.then((cleanup) => cleanup());
      instanceUpdate.then((cleanup) => cleanup());
      locationUpdate.then((cleanup) => cleanup());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // update store
  useEffect(() => {
    if (instances) {
      setListChecked(true);
      setInstances(instances);
    }
    if (tunnels) {
      setListChecked(true);
      setTunnels(tunnels);
    }
  }, [
    instances,
    setInstances,
    tunnels,
    setTunnels,
    setListChecked,
    openDeadConDroppedModal,
  ]);

  // navigate to carousel on first app Launch
  useEffect(() => {
    if (!location.pathname.includes(routes.client.carousel) && firstLaunch) {
      navigate(routes.client.carousel, { replace: true });
    }
  }, [firstLaunch, navigate, location.pathname]);

  useEffect(() => {
    if (listChecked && instances?.length === 0 && tunnels?.length === 0) {
      navigate(routes.client.carousel, { replace: true });
    }
  }, [navigate, listChecked, instances, tunnels]);

  return (
    <>
      <Outlet />
      <DeadConDroppedModal />
      <ClientSideBar />
    </>
  );
};
