import './style.scss';
import type { ComponentType } from 'react';
import { Tabs } from '../../shared/components/Tabs/Tabs';
import { FullPage } from '../../shared/layouts/FullPage/FullPage';
import { PlaygroundTestFactorSelector } from './components/PlaygroundTestFactorSelector/PlaygroundTestFactorSelector';
import { PlaygroundTestLoaderSpinner } from './components/PlaygroundTestLoaderSpinner/PlaygroundTestLoaderSpinner';
import { PlaygroundTestMfaSelector } from './components/PlaygroundTestMfaSelector/PlaygroundTestMfaSelector';
import { PlaygroundTestSelectionItem } from './components/PlaygroundTestSelectionItem/PlaygroundTestSelectionItem';

const tests: ComponentType[] = [
  PlaygroundTestMfaSelector,
  PlaygroundTestFactorSelector,
  PlaygroundTestSelectionItem,
  PlaygroundTestLoaderSpinner,
];

export const PlaygroundIndex = () => {
  return (
    <FullPage id="playground-index">
      <div id="playground-nav">
        <div className="track">
          <Tabs items={[{ title: 'General', active: true, onClick: () => {} }]} />
        </div>
      </div>
      <div className="main-track">
        {tests.map((Test, index) => (
          <Test key={index} />
        ))}
      </div>
    </FullPage>
  );
};
