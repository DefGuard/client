import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { ConfigureMfaVerificatorFactorSelector } from '../../components/ConfigureMfaVerificatorFactorSelector/ConfigureMfaVerificatorFactorSelector';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import type { MfaVerificationMethod } from '../../types';
import '../style.scss';
import './style.scss';

interface Props {
  onCancel: () => void;
}

/** shown only when the session offers more than one method */
export const ConfigureSelectVerificationStep = ({ onCancel }: Props) => {
  const methods = useConfigureMfaStore((s) => s.verificationMethods);

  const [selected, setSelected] = useState<MfaVerificationMethod | undefined>(methods[0]);

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: discardMfaConfiguration,
    onSettled: onCancel,
  });

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
          text="Cancel"
          variant={ButtonVariant.Secondary}
          loading={isCancelling}
          onClick={() => {
            cancel();
          }}
        />
        <div className="right">
          <Button
            text="Continue"
            variant={ButtonVariant.Primary}
            disabled={selected === undefined || isCancelling}
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
