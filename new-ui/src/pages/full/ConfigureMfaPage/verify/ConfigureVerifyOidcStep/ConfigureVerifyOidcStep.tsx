import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { ExternalProviderButton } from '../../../../../shared/components/ExternalProviderButton/ExternalProviderButton';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { findOpenIdProvider, openIdProviderName } from '../../../../../shared/utils/mfa';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';
import { useConfigureVerifyOidc } from './useConfigureVerifyOidc';
import '../style.scss';

interface Props {
  onSessionExpired: () => void;
}

export const ConfigureVerifyOidcStep = ({ onSessionExpired }: Props) => {
  const instance = useConfigureMfaStore((s) => s.instance) ?? undefined;
  const providerName = openIdProviderName(instance);
  const { start, abort, isOpening, isPolling, error } = useConfigureVerifyOidc({
    onSessionExpired,
    autoStart: false,
  });

  const handleBack = async () => {
    // Core holds one pending attempt per session, so abort it before another method starts
    await abort();
    useConfigureMfaStore.getState().backFromVerification();
  };

  return (
    <FullPage
      id="configure-verify-oidc-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Verify with OpenID" />
      <p className="description">
        <span>
          {isPolling
            ? 'Complete the sign-in in your browser. This page will update automatically.'
            : `A browser window will open for you to sign in with ${providerName}.`}
        </span>
      </p>
      {isPresent(error) && <p className="error">{error}</p>}
      <div className="oidc-action">
        <ExternalProviderButton
          text={`Authenticate with ${providerName}`}
          provider={findOpenIdProvider(instance) ?? 'custom'}
          loading={isOpening || isPolling}
          onClick={() => {
            void start();
          }}
        />
      </div>
      <Controls>
        <Button
          text="Back"
          variant={ButtonVariant.Secondary}
          onClick={() => {
            void handleBack();
          }}
        />
      </Controls>
    </FullPage>
  );
};
