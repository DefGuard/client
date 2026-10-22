import './style.scss';
import { useEffect } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { LoaderSpinner } from '../../../../../shared/components/LoaderSpinner/LoaderSpinner';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { FullPage } from '../../../../../shared/layouts/FullPage/FullPage';
import { ThemeSpacing } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { useConfigureMfaStore } from '../../hooks/useConfigureMfaStore';
import { useLeaveVerification } from '../../hooks/useLeaveVerification';
import { useConfigureVerifyOidc } from './useConfigureVerifyOidc';

interface Props {
  onCancel: () => void;
  onSessionExpired: () => void;
}

/** reached from the provider button on the picker, so the browser opens on mount */
export const ConfigureVerifyOidcStep = ({ onCancel, onSessionExpired }: Props) => {
  const { leave, isLeaving } = useLeaveVerification(onCancel);
  const { abort, error } = useConfigureVerifyOidc({
    onSessionExpired,
    autoStart: true,
  });

  useEffect(() => {
    if (isPresent(error)) {
      useConfigureMfaStore.getState().backFromVerification(error);
    }
  }, [error]);

  const handleLeave = async () => {
    // Core holds one pending attempt per session, so abort it before another method starts
    await abort();
    leave();
  };

  return (
    <FullPage id="configure-verify-oidc-step" hideScrollContainer>
      <div className="content">
        <LoaderSpinner variant="primary" size={28} />
        <SizedBox height={ThemeSpacing.Xl2} />
        <p className="title">Continue in your browser</p>
        <SizedBox height={ThemeSpacing.Sm} />
        <p className="subtitle">
          Once verified, the process will continue automatically here.
        </p>
        <SizedBox height={ThemeSpacing.Xl2} />
        <Button
          text="Cancel"
          variant={ButtonVariant.Secondary}
          loading={isLeaving}
          onClick={() => {
            void handleLeave();
          }}
        />
      </div>
    </FullPage>
  );
};
