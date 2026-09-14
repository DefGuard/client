import { useNavigate } from '@tanstack/react-router';
import { useCallback } from 'react';
import { ConfigureMfaTimeoutProvider } from './components/ConfigureMfaTimeoutProvider';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from './hooks/useConfigureMfaStore';
import { ConfigureMfaVerify } from './verify/ConfigureMfaVerify';
import { ConfigureMfaWizard } from './wizard/ConfigureMfaWizard';

export const ConfigureMfaPage = () => {
  const navigate = useNavigate();
  const authorized = useConfigureMfaStore((s) => s.authorized);

  const leave = useCallback(() => {
    navigate({ to: '/full/add' });
  }, [navigate]);

  const handleSessionExpired = useCallback(() => {
    void discardMfaConfiguration();
    leave();
  }, [leave]);

  return (
    <ConfigureMfaTimeoutProvider>
      {authorized ? (
        <ConfigureMfaWizard onCancel={leave} onSessionExpired={handleSessionExpired} />
      ) : (
        <ConfigureMfaVerify onCancel={leave} onSessionExpired={handleSessionExpired} />
      )}
    </ConfigureMfaTimeoutProvider>
  );
};
