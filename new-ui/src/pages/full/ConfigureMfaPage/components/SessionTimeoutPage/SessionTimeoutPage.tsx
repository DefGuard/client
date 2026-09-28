import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { EmptyState } from '../../../../../shared/components/EmptyState/EmptyState';
import { EmptyIcon } from '../../../../../shared/components/EmptyState/types';
import { discardMfaConfiguration } from '../../hooks/useConfigureMfaStore';

const DESCRIPTION = `Sorry, you have exceeded the time limit to complete the process. Please try again. If you need assistance, please watch our guide or contact your administrator.`;

export const SessionTimeoutPage = () => {
  const navigate = useNavigate();

  return (
    <div id="configure-mfa-session-timeout">
      <EmptyState
        title="Session timed out"
        subtitle={DESCRIPTION}
        icon={EmptyIcon.SessionTimeout}
        primaryAction={{
          text: 'Try again',
          variant: ButtonVariant.Primary,
          onClick: () => {
            // Reset once the page is gone, or the flow re-renders on an empty store.
            void navigate({ to: '/full/add', replace: true }).then(() => {
              void discardMfaConfiguration();
            });
          },
        }}
      />
    </div>
  );
};
