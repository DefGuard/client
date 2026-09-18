import { createFileRoute, redirect } from '@tanstack/react-router';
import { ConfigureMfaPage } from '../../pages/full/ConfigureMfaPage/ConfigureMfaPage';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../../pages/full/ConfigureMfaPage/hooks/useConfigureMfaStore';
import { isPresent } from '../../shared/utils/isPresent';

export const Route = createFileRoute('/full/configure-mfa')({
  // Gating on the factors left to set up would evict the user from the recovery codes step.
  beforeLoad: () => {
    if (!isPresent(useConfigureMfaStore.getState().sessionId)) {
      void discardMfaConfiguration();
      throw redirect({ to: '/full/add' });
    }
  },
  component: ConfigureMfaPage,
});
