import { useMemo, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { ConfigureMfaVerificatorFactorSelector } from '../../components/ConfigureMfaVerificatorFactorSelector/ConfigureMfaVerificatorFactorSelector';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';
import type { MfaVerificationMethod } from '../../types';
import { verificationMethodsOf } from '../../utils';
import '../style.scss';
import './style.scss';

/** Shown only when more than one code factor is configured. */
export const ConfigureSelectVerificationStep = () => {
  const configuredMethods = useConfigureMfaStore((s) => s.configuredMethods);

  const methods = useMemo(
    () => verificationMethodsOf(configuredMethods),
    [configuredMethods],
  );

  const [selected, setSelected] = useState<MfaVerificationMethod | undefined>(methods[0]);

  return (
    <FullPage
      id="configure-select-verification-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Additional verification required" />
      <p className="description">
        <span>{`To continue, please select one of your already set up methods. This is an extra security step to confirm your identity.`}</span>
      </p>
      <div className="methods">
        {methods.map((method) => (
          <ConfigureMfaVerificatorFactorSelector
            key={method}
            mfaMethod={method}
            selected={selected === method}
            onClick={() => {
              setSelected(method);
            }}
          />
        ))}
      </div>
      <Controls>
        <Button
          text="Back"
          variant={ButtonVariant.Secondary}
          onClick={() => {
            useConfigureMfaStore.getState().backToSelection();
          }}
        />
        <div className="right">
          <Button
            text="Continue"
            variant={ButtonVariant.Primary}
            disabled={selected === undefined}
            onClick={() => {
              if (selected === undefined) return;
              useConfigureMfaStore.getState().selectVerificationMethod(selected);
            }}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
