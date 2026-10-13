import './style.scss';
import { useState } from 'react';
import { FactorSelector } from '../../../../shared/components/FactorSelector/FactorSelector';
import { SelectionItemVariant } from '../../../../shared/components/SelectionItem/types';
import { MfaMethod, type MfaMethodValue } from '../../../../shared/rust-api/types';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const methods = Object.values(MfaMethod);

export const PlaygroundTestFactorSelector = () => {
  const [radio, setRadio] = useState<MfaMethodValue>();
  const [checked, setChecked] = useState<MfaMethodValue[]>([]);

  return (
    <PlaygroundCard>
      <div className="playground-test-factor-selector">
        <h3>Radio</h3>
        <div className="track">
          {methods.map((method) => (
            <FactorSelector
              key={method}
              method={method}
              selected={radio === method}
              onClick={() => setRadio(method)}
            />
          ))}
        </div>
        <h3>Checkbox</h3>
        <div className="track">
          {methods.map((method) => (
            <FactorSelector
              key={method}
              method={method}
              variant={SelectionItemVariant.Checkbox}
              selected={checked.includes(method)}
              onClick={() =>
                setChecked((current) =>
                  current.includes(method)
                    ? current.filter((item) => item !== method)
                    : [...current, method],
                )
              }
            />
          ))}
        </div>
        <h3>OpenID with provider name</h3>
        <div className="track">
          <FactorSelector
            method={MfaMethod.Oidc}
            instance={{ openid_display_name: 'Acme SSO', openid_provider_kind: 'custom' }}
            selected={false}
          />
        </div>
        <h3>Disabled</h3>
        <div className="track">
          <FactorSelector method={MfaMethod.Totp} selected={false} disabled />
          <FactorSelector method={MfaMethod.Fido2} selected disabled />
        </div>
      </div>
    </PlaygroundCard>
  );
};
