import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { ExternalProviderButton } from '../../../../../shared/components/ExternalProviderButton/ExternalProviderButton';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { ConfigureFactorsSource, MfaMethod } from '../../../../../shared/rust-api/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  type ClientAuthorizingMethod,
  findOpenIdProvider,
  openIdProviderName,
} from '../../../../../shared/utils/mfa';
import { ConfigureMfaVerificatorFactorSelector } from '../../components/ConfigureMfaVerificatorFactorSelector/ConfigureMfaVerificatorFactorSelector';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import '../style.scss';
import './style.scss';

interface Props {
  onCancel: () => void;
}

/** shown when the session offers more than one method, or only OpenID */
export const ConfigureSelectVerificationStep = ({ onCancel }: Props) => {
  const navigate = useNavigate();
  const methods = useConfigureMfaStore((s) => s.verificationMethods);
  const error = useConfigureMfaStore((s) => s.verificationError);
  const fromAddPage = useConfigureMfaStore(
    (s) => s.source === ConfigureFactorsSource.AddPage,
  );
  const instance = useConfigureMfaStore((s) => s.instance) ?? undefined;

  const [selected, setSelected] = useState<ClientAuthorizingMethod | undefined>(
    methods[0],
  );

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    // the source is read before the discard resets the store
    mutationFn: async () => {
      const { source } = useConfigureMfaStore.getState();
      await discardMfaConfiguration();
      return source;
    },
    onSettled: (source) => {
      if (source === ConfigureFactorsSource.AddPage) {
        void navigate({ to: '/full/add/mfa' });
        return;
      }
      onCancel();
    },
  });

  return (
    <FullPage
      id="configure-select-verification-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Verify your identity to add a new MFA method" />
      <p className="description">
        <span>{`For security, verify your identity using one of your existing MFA methods. You'll be able to add a new method after verification.`}</span>
      </p>
      {isPresent(error) && <p className="error">{error}</p>}
      <div className="label">
        <p>Select an existing method</p>
      </div>
      <div className="methods">
        {methods.map((method) => (
          <ConfigureMfaVerificatorFactorSelector
            key={method}
            mfaMethod={method}
            instance={instance}
            selected={selected === method}
            onClick={() => {
              setSelected(method);
            }}
          />
        ))}
      </div>
      <Controls>
        <Button
          text={fromAddPage ? 'Back' : 'Cancel'}
          variant={ButtonVariant.Secondary}
          loading={isCancelling}
          onClick={() => {
            cancel();
          }}
        />
        <div className="right">
          {selected === MfaMethod.Oidc && (
            <ExternalProviderButton
              text={`Authenticate with ${openIdProviderName(instance)}`}
              provider={findOpenIdProvider(instance) ?? 'custom'}
              disabled={isCancelling}
              onClick={() => {
                useConfigureMfaStore.getState().selectVerificationMethod(MfaMethod.Oidc);
              }}
            />
          )}
          {selected !== MfaMethod.Oidc && (
            <Button
              text="Verify identity"
              variant={ButtonVariant.Primary}
              disabled={selected === undefined || isCancelling}
              onClick={() => {
                if (selected === undefined) return;
                useConfigureMfaStore.getState().selectVerificationMethod(selected);
              }}
            />
          )}
        </div>
      </Controls>
    </FullPage>
  );
};
