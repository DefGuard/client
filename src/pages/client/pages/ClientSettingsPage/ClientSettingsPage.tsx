import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { GlobalSettingsTab } from './components/GlobalSettingsTab/GlobalSettingsTab';

export const ClientSettingsPage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.settingsPage;
  return (
    <section id="client-settings-page" className="client-page">
      <header>
        <h1>{pageLL.title()}</h1>
      </header>
      <Card id="settings-card">
        <GlobalSettingsTab />
      </Card>
    </section>
  );
};
