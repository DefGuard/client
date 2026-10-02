import './style.scss';
import { useState } from 'react';
import { ConnectButton } from '../../../../shared/components/LocationCard/components/ConnectButton/ConnectButton';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const noop = () => {};

export const PlaygroundTestConnectButton = () => {
  const [active, setActive] = useState(false);

  return (
    <PlaygroundCard>
      <div className="playground-test-connect-button">
        <h3>Disconnected</h3>
        <div className="track">
          <ConnectButton active={false} onClick={noop} />
        </div>
        <h3>Connected</h3>
        <div className="track">
          <ConnectButton active onClick={noop} />
        </div>
        <h3>Disconnected (disabled)</h3>
        <div className="track">
          <ConnectButton active={false} disabled onClick={noop} />
        </div>
        <h3>Connected (disabled)</h3>
        <div className="track">
          <ConnectButton active disabled onClick={noop} />
        </div>
        <h3>Interactive (active: {active ? 'true' : 'false'})</h3>
        <div className="track">
          <ConnectButton active={active} onClick={() => setActive((v) => !v)} />
        </div>
      </div>
    </PlaygroundCard>
  );
};
