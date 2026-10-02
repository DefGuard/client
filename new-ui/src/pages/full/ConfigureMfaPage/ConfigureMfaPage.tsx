import { useNavigate } from '@tanstack/react-router';
import { useCallback } from 'react';
import { isPresent } from '../../../shared/utils/isPresent';
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
  // a late authorization on the selection screen waits for the picks
  const inWizard = useConfigureMfaStore(
    (s) => s.authorized && isPresent(s.selectedMethods),
  );
  const handleSessionExpired = useConfigureMfaSessionExpired();

  const leave = useCallback(() => {
    navigate({ to: '/full/add' });
  }, [navigate]);

  return (
    <>
      {inWizard && (
        <ConfigureMfaWizard onCancel={leave} onSessionExpired={handleSessionExpired} />
      )}
      {!inWizard && (
        <ConfigureMfaVerify onCancel={leave} onSessionExpired={handleSessionExpired} />
      )}
    </>
  );
};
