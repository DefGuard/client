import { useNavigate } from '@tanstack/react-router';
import { useCallback } from 'react';
import { isPresent } from '../../../shared/utils/isPresent';
import {
  ConfigureMfaTimeoutProvider,
  useConfigureMfaSessionExpired,
} from './components/ConfigureMfaTimeoutProvider';
import { useConfigureMfaStore } from './hooks/useConfigureMfaStore';
import { ConfigureMfaVerify } from './verify/ConfigureMfaVerify';
import { ConfigureSelectMethodsStep } from './verify/ConfigureSelectMethodsStep/ConfigureSelectMethodsStep';
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
  const methodsSelected = useConfigureMfaStore((s) => isPresent(s.selectedMethods));
  const handleSessionExpired = useConfigureMfaSessionExpired();

  const leave = useCallback(() => {
    navigate({ to: '/full/add' });
  }, [navigate]);

  return (
    <>
      {!authorized && (
        <ConfigureMfaVerify onCancel={leave} onSessionExpired={handleSessionExpired} />
      )}
      {authorized && !methodsSelected && <ConfigureSelectMethodsStep onCancel={leave} />}
      {authorized && methodsSelected && (
        <ConfigureMfaWizard onCancel={leave} onSessionExpired={handleSessionExpired} />
      )}
    </>
  );
};
