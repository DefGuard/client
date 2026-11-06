import './style.scss';
import { useState } from 'react';
import { FactorSelector } from '../../../../shared/components/FactorSelector/FactorSelector';
import { SelectionItemVariant } from '../../../../shared/components/SelectionItem/types';
import { MfaMethod, type MfaMethodValue } from '../../../../shared/rust-api/types';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';
import { PlaygroundGroup } from '../PlaygroundGroup/PlaygroundGroup';

const availableMethods: MfaMethodValue[] = [
  MfaMethod.Totp,
  MfaMethod.Email,
  MfaMethod.Fido2,
];

export const PlaygroundTestFactorSelector = () => {
  const [selected, setSelected] = useState<MfaMethodValue[]>([MfaMethod.Totp]);

  const toggle = (method: MfaMethodValue) => {
    setSelected((current) =>
      current.includes(method)
        ? current.filter((picked) => picked !== method)
        : [...current, method],
    );
  };

  return (
    <PlaygroundCard>
      <div className="playground-test-factor-selector">
        <h3>FactorSelector</h3>
        <PlaygroundGroup title="Available methods">
          {availableMethods.map((method) => (
            <FactorSelector
              key={method}
              method={method}
              variant={SelectionItemVariant.Checkbox}
              selected={selected.includes(method)}
              onClick={() => {
                toggle(method);
              }}
            />
          ))}
        </PlaygroundGroup>
        <PlaygroundGroup title="Not configurable">
          <FactorSelector
            method={MfaMethod.Oidc}
            variant={SelectionItemVariant.Checkbox}
            badge="Configured"
            selected={false}
            disabled
          />
          <FactorSelector
            method={MfaMethod.MobileApprove}
            variant={SelectionItemVariant.Checkbox}
            badge="Not configurable"
            selected={false}
            disabled
          />
        </PlaygroundGroup>
      </div>
    </PlaygroundCard>
  );
};
