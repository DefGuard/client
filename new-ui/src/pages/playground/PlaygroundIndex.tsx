import './style.scss';
import type { ComponentType } from 'react';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { Tabs } from '../../shared/components/Tabs/Tabs';
import { FullPage } from '../../shared/layouts/FullPage/FullPage';
import { PlaygroundSnackbarTest } from './components/PlaygroundSnackbarTest/PlaygroundSnackbarTest';
import { PlaygroundTestConnectButton } from './components/PlaygroundTestConnectButton/PlaygroundTestConnectButton';
import { PlaygroundTestFactorSelector } from './components/PlaygroundTestFactorSelector/PlaygroundTestFactorSelector';
import { PlaygroundTestInstanceSelector } from './components/PlaygroundTestInstanceSelector/PlaygroundTestInstanceSelector';
import { PlaygroundTestMenu } from './components/PlaygroundTestMenu/PlaygroundTestMenu';
import { PlaygroundTestMfaSelector } from './components/PlaygroundTestMfaSelector/PlaygroundTestMfaSelector';
import { PlaygroundTestMfaSettingsSection } from './components/PlaygroundTestMfaSettingsSection/PlaygroundTestMfaSettingsSection';
import { PlaygroundTestMfaVerificatorFactorSelector } from './components/PlaygroundTestMfaVerificatorFactorSelector/PlaygroundTestMfaVerificatorFactorSelector';
import { PlaygroundTestOpenIdProviders } from './components/PlaygroundTestOpenIdProviders/PlaygroundTestOpenIdProviders';
import { PlaygroundTestSelect } from './components/PlaygroundTestSelect';
import { PlaygroundTestSelectionItem } from './components/PlaygroundTestSelectionItem/PlaygroundTestSelectionItem';

type PlaygroundTab = 'connection' | 'mfa-setup' | 'shared';

type PlaygroundTabConfig = {
  key: PlaygroundTab;
  title: string;
  tests: ComponentType[];
};

const tabs: PlaygroundTabConfig[] = [
  {
    key: 'connection',
    title: 'Connection',
    tests: [
      PlaygroundTestConnectButton,
      PlaygroundTestMfaSelector,
      PlaygroundTestOpenIdProviders,
      PlaygroundTestMfaSettingsSection,
    ],
  },
  {
    key: 'mfa-setup',
    title: 'MFA setup',
    tests: [PlaygroundTestMfaVerificatorFactorSelector, PlaygroundTestInstanceSelector],
  },
  {
    key: 'shared',
    title: 'Shared components',
    tests: [
      PlaygroundTestSelect,
      PlaygroundTestSelectionItem,
      PlaygroundTestFactorSelector,
      PlaygroundSnackbarTest,
      PlaygroundTestMenu,
    ],
  },
];

type Store = {
  activeTab: PlaygroundTab;
  setActiveTab: (tab: PlaygroundTab) => void;
};

const usePlaygroundStore = create<Store>()(
  persist(
    (set) => ({
      activeTab: 'connection',
      setActiveTab: (activeTab) => set({ activeTab }),
    }),
    {
      name: 'playground-store',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) => ({ activeTab: state.activeTab }),
    },
  ),
);

export const PlaygroundIndex = () => {
  const activeTab = usePlaygroundStore((s) => s.activeTab);
  const setActiveTab = usePlaygroundStore((s) => s.setActiveTab);
  // sessionStorage can hold a key from an older build that matches no tab
  const active = tabs.find((tab) => tab.key === activeTab) ?? tabs[0];

  return (
    <FullPage id="playground-index">
      <div id="playground-nav">
        <div className="track">
          <Tabs
            items={tabs.map((tab) => ({
              title: tab.title,
              active: tab.key === active.key,
              onClick: () => setActiveTab(tab.key),
            }))}
          />
        </div>
      </div>
      <div className="main-track">
        {active.tests.map((Test, index) => (
          <Test key={`${active.key}-${index}`} />
        ))}
      </div>
    </FullPage>
  );
};
