import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../i18n/i18n-react';
import { useToaster } from '../../shared/defguard-ui/hooks/toasts/useToaster';
import { routes } from '../../shared/routes';
import { clientApi } from './clientAPI/clientApi';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
import { useClientFlags } from './hooks/useClientFlags';
import { useClientStore } from './hooks/useClientStore';
import { clientQueryKeys } from './query';
import { TauriEventKey } from './types';

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
    const subs: UnlistenFn[] = [];

    listen(TauriEventKey.INSTANCE_UPDATE, () => {
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
    }).then((cleanup) => {
      subs.push(cleanup);
    });

    listen(TauriEventKey.LOCATION_UPDATE, () => {
      const invalidate = [clientQueryKeys.getLocations, clientQueryKeys.getTunnels];
      invalidate.forEach((key) =>
        queryClient.invalidateQueries({
          queryKey: [key],
        }),
      );
    }).then((cleanup) => {
      subs.push(cleanup);
    });

    listen(TauriEventKey.CONNECTION_CHANGED, () => {
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
    }).then((cleanup) => {
      subs.push(cleanup);
    });

    listen(TauriEventKey.CONFIG_CHANGED, (data) => {
      const instance = data.payload as string;
      toaster.info(LL.common.messages.configChanged({ instance }));
    }).then((cleanup) => {
      subs.push(cleanup);
    });

    return () => {
      subs.forEach((sub) => sub());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryClient]);

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
  }, [instances, setInstances, tunnels, setTunnels, setListChecked]);

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
      <ClientSideBar />
    </>
  );
};
