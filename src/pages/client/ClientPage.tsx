import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { debug, error } from 'tauri-plugin-log-api';

import { useI18nContext } from '../../i18n/i18n-react';
import { clientApi } from './clientAPI/clientApi';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
import { LocationsList } from './components/LocationsList/LocationsList';
import { AddInstanceModal } from './components/modals/AddInstanceModal/AddInstanceModal';
import { StatsFilterSelect } from './components/StatsFilterSelect/StatsFilterSelect';
import { StatsLayoutSelect } from './components/StatsLayoutSelect/StatsLayoutSelect';
import { useClientStore } from './hooks/useClientStore';
import { clientQueryKeys } from './query';

const { getInstances } = clientApi;

export const ClientPage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client;
  const setInstances = useClientStore((state) => state.setInstances);

  useQuery({
    queryKey: [clientQueryKeys.getInstances],
    queryFn: getInstances,
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    retry: (retryCount) => {
      error(`Retrieving instances failed ${retryCount} times.`);
      if (retryCount > 5) {
        return false;
      }
      return true;
    },
    onSuccess: (res) => {
      setInstances(res);
      debug(`Retrieved ${res.length} instances`);
    },
    onError: (err) => {
      error(`Error retrieving instances: ${String(err)}`);
    },
  });

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
