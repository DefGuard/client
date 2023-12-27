import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { AddTunnelGuide } from './AddTunnelGuide/AddTunnelGuide';
import { AddTunnelFormCard } from './components/AddTunnelFormCard/AddTunnelFormCard';

export const ClientAddTunnelPage = () => {
  const { LL } = useI18nContext();
  return (
    <section className="client-page" id="client-add-tunnel-page">
      <header>
        <h1>{LL.pages.client.pages.addTunnelPage.title()}</h1>
      </header>
      <div className="content">
        <AddTunnelFormCard />
        <AddTunnelGuide />
      </div>
    </section>
  );
};
