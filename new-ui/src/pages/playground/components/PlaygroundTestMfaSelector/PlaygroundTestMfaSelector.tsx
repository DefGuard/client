import './style.scss';
import { useState } from 'react';
import { MfaSelector } from '../../../../shared/components/LocationCard/components/MfaSelector/MfaSelector';
import { MfaMethod, type MfaMethodValue } from '../../../../shared/rust-api/types';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';
import { PlaygroundGroup } from '../PlaygroundGroup/PlaygroundGroup';

const pickMethods: MfaMethodValue[] = [MfaMethod.Totp, MfaMethod.Email];
const configurableMethods: MfaMethodValue[] = [MfaMethod.MobileApprove, MfaMethod.Fido2];

export const PlaygroundTestMfaSelector = () => {
  const [active, setActive] = useState<MfaMethodValue>(MfaMethod.Totp);
  const [queued, setQueued] = useState<MfaMethodValue[]>([]);

  const toggleQueued = (method: MfaMethodValue) => {
    setQueued((current) =>
      current.includes(method)
        ? current.filter((picked) => picked !== method)
        : [...current, method],
    );
  };

  return (
    <PlaygroundCard>
      <div className="playground-test-mfa-selector">
        <h3>MfaSelector</h3>
        <PlaygroundGroup title="Configured">
          {pickMethods.map((method) => (
            <MfaSelector
              key={method}
              factor={method}
              active={active === method}
              isDefault={method === MfaMethod.Totp}
              isSelectable
              onClick={() => {
                setActive(method);
              }}
            />
          ))}
        </PlaygroundGroup>
        <PlaygroundGroup title="Can be configured">
          {configurableMethods.map((method) => (
            <MfaSelector
              key={method}
              factor={method}
              configured={false}
              configurable
              selected={queued.includes(method)}
              isSelectable
              onClick={() => {
                toggleQueued(method);
              }}
            />
          ))}
        </PlaygroundGroup>
        <PlaygroundGroup title="Not selectable">
          <MfaSelector
            factor={MfaMethod.Biometric}
            configured={false}
            isSelectable={false}
          />
        </PlaygroundGroup>
      </div>
    </PlaygroundCard>
  );
};
