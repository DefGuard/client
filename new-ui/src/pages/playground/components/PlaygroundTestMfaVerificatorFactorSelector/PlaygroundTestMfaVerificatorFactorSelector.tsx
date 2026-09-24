import './style.scss';
import { useState } from 'react';
import { MfaMethod, type MfaMethodValue } from '../../../../shared/rust-api/types';
import { ConfigureMfaVerificatorFactorSelector } from '../../../full/ConfigureMfaPage/components/ConfigureMfaVerificatorFactorSelector/ConfigureMfaVerificatorFactorSelector';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const factors: MfaMethodValue[] = [MfaMethod.Totp, MfaMethod.Email];

export const PlaygroundTestMfaVerificatorFactorSelector = () => {
  const [selected, setSelected] = useState<MfaMethodValue>();

  return (
    <PlaygroundCard>
      <div className="playground-test-mfa-verificator-factor-selector">
        <h3>Default</h3>
        <div className="track">
          {factors.map((factor) => (
            <ConfigureMfaVerificatorFactorSelector
              key={factor}
              mfaMethod={factor}
              selected={false}
              onClick={() => {}}
            />
          ))}
        </div>
        <h3>Selected</h3>
        <div className="track">
          {factors.map((factor) => (
            <ConfigureMfaVerificatorFactorSelector
              key={factor}
              mfaMethod={factor}
              selected
              onClick={() => {}}
            />
          ))}
        </div>
        <h3>Disabled</h3>
        <div className="track">
          {factors.map((factor) => (
            <ConfigureMfaVerificatorFactorSelector
              key={factor}
              mfaMethod={factor}
              selected={false}
              disabled
              onClick={() => {}}
            />
          ))}
        </div>
        <h3>Interactive (selected: {selected ?? 'none'})</h3>
        <div className="track">
          {factors.map((factor) => (
            <ConfigureMfaVerificatorFactorSelector
              key={factor}
              mfaMethod={factor}
              selected={selected === factor}
              onClick={() => setSelected(factor)}
            />
          ))}
        </div>
      </div>
    </PlaygroundCard>
  );
};
