import './style.scss';

import { useQuery } from '@tanstack/react-query';

import { useI18nContext } from '../../i18n/i18n-react';
import { clientApi } from './clientAPI/clientApi';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
import { LocationsList } from './components/LocationsList/LocationsList';
import { AddInstanceModal } from './components/modals/AddInstanceModal/AddInstanceModal';
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
    onSuccess: (res) => {
      setInstances(res);
    },
  });

  return (
    <>
      <section id="client-page">
        <header>
          <h1>{pageLL.title()}</h1>
        </header>
        <LocationsList />
      </section>
      <ClientSideBar />
      <AddInstanceModal />
    </>
  );
};
