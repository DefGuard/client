import { useI18nContext } from '../../../../../../i18n/i18n-react';

export const GlobalSettingsTab = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global;

  return (
    <div id="global-settings-tab">
      <section>
        <h2>{localLL.tray.title()}</h2>
      </section>
      <section>
        <h2>{localLL.logging.title()}</h2>
      </section>
      <section>
        <h2>{localLL.theme.title()}</h2>
      </section>
    </div>
  );
};
