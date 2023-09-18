import './style.scss';

import { useI18nContext } from '../../i18n/i18n-react';
import { ClientSideBar } from './components/ClientSideBar/ClientSideBar';
import { LocationsList } from './components/LocationsList/LocationsList';
import { AddInstanceModal } from './components/modals/AddInstanceModal/AddInstanceModal';

export const ClientPage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client;
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
