import { useNavigate } from '@tanstack/react-router';
import { useCallback } from 'react';
import {
  ConfigureMfaTimeoutProvider,
  useConfigureMfaSessionExpired,
} from './components/ConfigureMfaTimeoutProvider';
import { useConfigureMfaStore } from './hooks/useConfigureMfaStore';
import { ConfigureMfaVerify } from './verify/ConfigureMfaVerify';
import { ConfigureMfaWizard } from './wizard/ConfigureMfaWizard';

export const ConfigureMfaPage = () => {
  return (
    <ConfigureMfaTimeoutProvider>
      <ConfigureMfaContent />
    </ConfigureMfaTimeoutProvider>
  );
};

const ConfigureMfaContent = () => {
  const navigate = useNavigate();
  const authorized = useConfigureMfaStore((s) => s.authorized);
  const handleSessionExpired = useConfigureMfaSessionExpired();

  const leave = useCallback(() => {
    navigate({ to: '/full/add' });
  }, [navigate]);

  return authorized ? (
    <ConfigureMfaWizard onCancel={leave} onSessionExpired={handleSessionExpired} />
  ) : (
    <ConfigureMfaVerify onCancel={leave} onSessionExpired={handleSessionExpired} />
  );
};
