import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { event } from '@tauri-apps/api';
import { UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { clientApi } from './clientAPI/clientApi';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
import { LocationsList } from './components/LocationsList/LocationsList';
import { AddInstanceModal } from './components/modals/AddInstanceModal/AddInstanceModal';
import { StatsFilterSelect } from './components/StatsFilterSelect/StatsFilterSelect';
import { StatsLayoutSelect } from './components/StatsLayoutSelect/StatsLayoutSelect';
import { useClientStore } from './hooks/useClientStore';
import { clientQueryKeys } from './query';
import { TauriEventKey } from './types';

const { getInstances } = clientApi;

export const ClientPage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client;
  const queryClient = useQueryClient();
  const setInstances = useClientStore((state) => state.setInstances);
  const { data: instances } = useQuery({
    queryFn: getInstances,
    queryKey: [clientQueryKeys.getInstances],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    refetchOnReconnect: true,
  });

  useEffect(() => {
    let instances_sub: UnlistenFn;
    let connection_sub: UnlistenFn;

    event
      .listen(TauriEventKey.INSTANCE_UPDATE, () => {
        const invalidate = [clientQueryKeys.getInstances, clientQueryKeys.getLocations];
        queryClient.invalidateQueries({
          predicate: (query) => invalidate.includes(query.queryKey[0] as string),
        });
      })
      .then((cleanup) => {
        instances_sub = cleanup;
      });

    event
      .listen(TauriEventKey.CONNECTION_CHANGED, () => {
        const invalidate = [
          clientQueryKeys.getLocations,
          clientQueryKeys.getConnections,
          clientQueryKeys.getActiveConnection,
          clientQueryKeys.getConnectionHistory,
          clientQueryKeys.getLocationStats,
          clientQueryKeys.getInstances,
        ];
        queryClient.invalidateQueries({
          predicate: (query) => invalidate.includes(query.queryKey[0] as string),
        });
      })
      .then((cleanup) => {
        connection_sub = cleanup;
      });

    return () => {
      instances_sub?.();
      connection_sub?.();
    };
  }, [queryClient]);

  // update store
  useEffect(() => {
    if (instances) {
      setInstances(instances);
    }
  }, [instances, setInstances]);

  return (
    <>
      <section id="client-page">
        <header>
          <h1>{pageLL.title()}</h1>
          <div className="options">
            <StatsFilterSelect />
            <StatsLayoutSelect />
          </div>
        </header>
        <LocationsList />
      </section>
      <ClientSideBar />
      <AddInstanceModal />
    </>
  );
};
