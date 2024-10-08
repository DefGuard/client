import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { GlobalSettingsTab } from './components/GlobalSettingsTab/GlobalSettingsTab';
import { InfoCard } from './components/InfoCard/InfoCard';
import { GlobalLogs } from './components/GlobalLogs/GlobalLogs';

export const ClientSettingsPage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.settingsPage;

  return (
    <section id="client-settings-page" className="client-page">
      <header>
        <h1>{pageLL.title()}</h1>
      </header>
      <div className="content">
        <div className="col">
          <Card id="settings-card">
            <GlobalSettingsTab />
          </Card>
          <GlobalLogs />
        </div>
        <InfoCard />
      </div>
    </section>
  );
};
