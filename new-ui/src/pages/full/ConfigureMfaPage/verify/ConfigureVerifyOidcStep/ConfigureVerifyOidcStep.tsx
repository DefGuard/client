import { useMutation } from '@tanstack/react-query';
import { useCallback } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { FullPageTitle } from '../../../../../shared/components/FullPageTitle/FullPageTitle';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import { useConfigureVerifyOidc } from './useConfigureVerifyOidc';
import '../style.scss';

interface Props {
  onCancel: () => void;
  onSessionExpired: () => void;
}

export const ConfigureVerifyOidcStep = ({ onCancel, onSessionExpired }: Props) => {
  const { start, abort, isOpening, isPolling, error } = useConfigureVerifyOidc({
    onSessionExpired,
  });

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: discardMfaConfiguration,
    onSettled: onCancel,
  });

  const handleBack = useCallback(async () => {
    // Core holds one pending attempt per session, so abort it before another method starts
    await abort();
    useConfigureMfaStore.getState().backFromVerification();
  }, [abort]);

  return (
    <FullPage
      id="configure-verify-oidc-step"
      className="configure-mfa-verify-page"
      hideScrollContainer
      withControls
    >
      <FullPageTitle title="Verify with OpenID" />
      <p className="description">
        {isPolling ? (
          <span>{`Complete the sign-in in your browser. This page will update automatically.`}</span>
        ) : (
          <span>{`Authenticate via your OpenID provider. A browser window will open for you to sign in.`}</span>
        )}
      </p>
      {isPresent(error) && <p className="error">{error}</p>}
      <div className="oidc-action">
        <Button
          text={isPolling ? 'Open again' : 'Auth with OpenID'}
          variant={ButtonVariant.Primary}
          loading={isOpening}
          disabled={isCancelling}
          onClick={() => {
            void start();
          }}
        />
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
            text="Back"
            variant={ButtonVariant.Outlined}
            disabled={isCancelling}
            onClick={() => {
              void handleBack();
            }}
          />
        </div>
      </Controls>
    </FullPage>
  );
};
