import './style.scss';
import { PlaygroundConfigureMfa } from './components/PlaygroundConfigureMfa/PlaygroundConfigureMfa';
import { PlaygroundSnackbarTest } from './components/PlaygroundSnackbarTest/PlaygroundSnackbarTest';
import { PlaygroundTestMenu } from './components/PlaygroundTestMenu/PlaygroundTestMenu';
import { PlaygroundTestSelect } from './components/PlaygroundTestSelect';

export const PlaygroundIndex = () => {
  return (
    <main id="playground-index">
      <div id="playground-nav">
        <div className="track">{/* tabs here */}</div>
      </div>
      <div className="main-track">
        <PlaygroundTestSelect />
        <PlaygroundSnackbarTest />
        <PlaygroundTestMenu />
        <PlaygroundConfigureMfa />
      </div>
    </main>
  );
};
