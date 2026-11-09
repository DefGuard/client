import { isPresent } from '../../../shared/utils/isPresent';
import {
  ConfigureMfaTimeoutProvider,
  useConfigureMfaSessionExpired,
} from './components/ConfigureMfaTimeoutProvider';
import { useConfigureMfaStore } from './hooks/useConfigureMfaStore';
import { useExitConfigureMfa } from './hooks/useExitConfigureMfa';
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
  const authorized = useConfigureMfaStore((s) => s.authorized);
  const methodsSelected = useConfigureMfaStore((s) => isPresent(s.selectedMethods));
  const handleSessionExpired = useConfigureMfaSessionExpired();
  const leave = useExitConfigureMfa();

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
