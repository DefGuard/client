import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';

import { routes } from '../../shared/routes';
import { clientApi } from './clientAPI/clientApi';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
import { useClientFlags } from './hooks/useClientFlags';
import { useClientStore } from './hooks/useClientStore';
import { clientQueryKeys } from './query';
import { TauriEventKey } from './types';

const { getInstances } = clientApi;

export const ClientPage = () => {
  const queryClient = useQueryClient();
  const setInstances = useClientStore((state) => state.setInstances);
  const navigate = useNavigate();
  const firstLaunch = useClientFlags((state) => state.firstStart);
  const location = useLocation();

  const { data: instances } = useQuery({
    queryFn: getInstances,
    queryKey: [clientQueryKeys.getInstances],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    const subs: UnlistenFn[] = [];

    listen(TauriEventKey.INSTANCE_UPDATE, () => {
      const invalidate = [clientQueryKeys.getInstances, clientQueryKeys.getLocations];
      invalidate.forEach((key) =>
        queryClient.invalidateQueries({
          queryKey: [key],
        }),
      );
    }).then((cleanup) => {
      subs.push(cleanup);
    });

    listen(TauriEventKey.LOCATION_UPDATE, () => {
      const invalidate = [clientQueryKeys.getLocations];
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
      ];
      invalidate.forEach((key) =>
        queryClient.invalidateQueries({
          queryKey: [key],
        }),
      );
    }).then((cleanup) => {
      subs.push(cleanup);
    });

    return () => {
      subs.forEach((sub) => sub());
    };
  }, [queryClient]);

  // update store
  useEffect(() => {
    if (instances) {
      setInstances(instances);
    }
  }, [instances, setInstances]);

  // navigate to carousel on first app Launch
  useEffect(() => {
    if (!location.pathname.includes(routes.client.carousel) && firstLaunch) {
      navigate(routes.client.carousel, { replace: true });
    }
  }, [firstLaunch, navigate, location.pathname]);

  return (
    <>
      <Outlet />
      <ClientSideBar />
    </>
  );
};
