import './style.scss';
import { useState } from 'react';
import { MfaSelector } from '../../../../shared/components/LocationCard/components/MfaSelector/MfaSelector';
import { MfaMethod, type MfaMethodValue } from '../../../../shared/rust-api/types';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const allFactors: MfaMethodValue[] = [
  MfaMethod.Totp,
  MfaMethod.Email,
  MfaMethod.Oidc,
  MfaMethod.MobileApprove,
  MfaMethod.Fido2,
  MfaMethod.Biometric,
];

const configuredFactors: MfaMethodValue[] = [
  MfaMethod.Totp,
  MfaMethod.Email,
  MfaMethod.Fido2,
];

export const PlaygroundTestMfaSelector = () => {
  const [selected, setSelected] = useState<MfaMethodValue>();

  return (
    <PlaygroundCard>
      <div className="playground-test-mfa-selector">
        <h3>All factors (configured)</h3>
        <div className="track">
          {allFactors.map((factor) => (
            <MfaSelector key={factor} factor={factor} isSelectable />
          ))}
        </div>
        <h3>All factors (not configured)</h3>
        <div className="track">
          {allFactors.map((factor) => (
            <MfaSelector
              key={factor}
              factor={factor}
              configured={false}
              isSelectable={false}
            />
          ))}
        </div>
        <h3>Selected</h3>
        <div className="track">
          {allFactors.map((factor) => (
            <MfaSelector key={factor} factor={factor} isSelectable selected />
          ))}
        </div>
        <h3>Default badge</h3>
        <div className="track">
          <MfaSelector factor={MfaMethod.Totp} isDefault isSelectable />
          <MfaSelector factor={MfaMethod.Email} isDefault isSelectable selected />
          <MfaSelector
            factor={MfaMethod.Fido2}
            isDefault
            configured={false}
            isSelectable={false}
          />
          <MfaSelector factor={MfaMethod.Biometric} isDefault isSelectable={false} />
        </div>
        <h3>Interactive (selected: {selected ?? 'none'})</h3>
        <div className="track">
          {allFactors.map((factor) => {
            const configured = configuredFactors.includes(factor);
            return (
              <MfaSelector
                key={factor}
                factor={factor}
                selected={selected === factor}
                isDefault={factor === MfaMethod.Totp}
                configured={configured}
                isSelectable={!configured && factor !== MfaMethod.Biometric}
                onClick={() => setSelected(factor)}
              />
            );
          })}
        </div>
      </div>
    </PlaygroundCard>
  );
};
