import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { Outlet } from 'react-router-dom';

import { clientApi } from './clientAPI/clientApi';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
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

    return () => {
      subs.forEach((sub) => sub());
    };
  }, [queryClient]);

  // update store
  useEffect(() => {
    if (instances) {
      setInstances(instances);
    }
    if (tunnels) {
      setTunnels(tunnels);
    }
  }, [instances, setInstances, tunnels, setTunnels]);

  return (
    <>
      <Outlet />
      <ClientSideBar />
    </>
  );
};
