import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { AddInstanceFormCard } from './components/AddInstanceFormCard/AddInstanceFormCard';
import { AddInstanceGuide } from './components/AddInstanceGuide/AddInstanceGuide';

export const ClientAddInstancePage = () => {
  const { LL } = useI18nContext();
  return (
    <section className="client-page" id="client-add-instance-page">
      <header>
        <h1>{LL.pages.client.pages.addInstancePage.title()}</h1>
      </header>
      <div className="content">
        <AddInstanceFormCard />
        <AddInstanceGuide />
      </div>
    </section>
  );
};
