import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { error as logError } from '@tauri-apps/plugin-log';
import { Snackbar } from '../../../../shared/providers/snackbar/snackbar';
import {
  isMfaConfigMissingToken,
  isMfaConfigUnsupported,
} from '../../../../shared/rust-api/mfaError';
import type { InstanceInfo } from '../../../../shared/rust-api/types';
import { startMfaConfiguration } from '../../ConfigureMfaPage/hooks/useConfigureMfaStore';

/** Opens a session and enters the wizard. On failure it stays put, so another instance
 *  can be tried. */
export const useStartMfaConfiguration = () => {
  const navigate = useNavigate();

  return useMutation({
    mutationFn: (instance: InstanceInfo) => startMfaConfiguration(instance),
    onSuccess: () => {
      navigate({
        to: '/full/configure-mfa',
      });
    },
    onError: (err) => {
      void logError(`MFA configuration start failed: ${err}`);
      if (isMfaConfigUnsupported(err)) {
        Snackbar.error(
          'This Defguard instance does not support configuring MFA from the client.',
        );
        return;
      }
      if (isMfaConfigMissingToken(err)) {
        Snackbar.error(
          'This device has no polling token; update the instance and try again.',
        );
        return;
      }
      Snackbar.error('Could not start MFA configuration.');
    },
  });
};
