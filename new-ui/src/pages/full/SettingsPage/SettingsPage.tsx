import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { Tabs } from '../../../shared/components/Tabs/Tabs';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { getAppConfigQueryOptions } from '../../../shared/rust-api/query';
import { ThemeSpacing } from '../../../shared/types';
import { GeneralTab } from './tabs/GeneralTab/GeneralTab';
import { StartupTab } from './tabs/StartupTab/StartupTab';
import { SettingsTab, type SettingsTabValue } from './types';

export const SettingsPage = () => {
  const [activeTab, setActiveTab] = useState<SettingsTabValue>(SettingsTab.General);
  const { data: appConfig } = useQuery(getAppConfigQueryOptions);

  if (!appConfig) return null;

  return (
    <FullPage id="settings-page-view" hideScrollContainer>
      <FullPageTitle title="Settings" spacing={ThemeSpacing.Xl} />
      <Tabs
        items={[
          {
            title: 'General',
            active: activeTab === SettingsTab.General,
            onClick: () => setActiveTab(SettingsTab.General),
          },
          {
            title: 'Startup & Auto-connect',
            active: activeTab === SettingsTab.Startup,
            onClick: () => setActiveTab(SettingsTab.Startup),
          },
        ]}
      />
      <div className="sections">
        {activeTab === SettingsTab.General && <GeneralTab appConfig={appConfig} />}
        {activeTab === SettingsTab.Startup && <StartupTab appConfig={appConfig} />}
      </div>
    </FullPage>
  );
};
