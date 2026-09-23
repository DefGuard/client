import './style.scss';
import { FullPage } from '../../shared/layouts/FullPage/FullPage';
import { PlaygroundSnackbarTest } from './components/PlaygroundSnackbarTest/PlaygroundSnackbarTest';
import { PlaygroundTestConnectButton } from './components/PlaygroundTestConnectButton/PlaygroundTestConnectButton';
import { PlaygroundTestMenu } from './components/PlaygroundTestMenu/PlaygroundTestMenu';
import { PlaygroundTestMfaSelector } from './components/PlaygroundTestMfaSelector/PlaygroundTestMfaSelector';
import { PlaygroundTestSelect } from './components/PlaygroundTestSelect';

export const PlaygroundIndex = () => {
  return (
    <FullPage id="playground-index">
      <div id="playground-nav">
        <div className="track">{/* tabs here */}</div>
      </div>
      <div className="main-track">
        <PlaygroundTestConnectButton />
        <PlaygroundTestMfaSelector />
        <PlaygroundTestSelect />
        <PlaygroundSnackbarTest />
        <PlaygroundTestMenu />
      </div>
    </FullPage>
  );
};
