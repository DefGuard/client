import './style.scss';
import type { ComponentType } from 'react';
import { Tabs } from '../../shared/components/Tabs/Tabs';
import { FullPage } from '../../shared/layouts/FullPage/FullPage';
import { PlaygroundTestLoaderSpinner } from './components/PlaygroundTestLoaderSpinner/PlaygroundTestLoaderSpinner';

const tests: ComponentType[] = [PlaygroundTestLoaderSpinner];

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
