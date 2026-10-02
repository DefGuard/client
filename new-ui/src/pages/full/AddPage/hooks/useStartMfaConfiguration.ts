import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  ConfigureFactorsSource,
  type InstanceInfo,
} from '../../../../shared/rust-api/types';
import { reportMfaConfigStartError } from '../../ConfigureMfaPage/hooks/reportMfaConfigStartError';
import { startMfaConfiguration } from '../../ConfigureMfaPage/hooks/useConfigureMfaStore';

/** Opens a session and enters the wizard. On failure it stays put, so another instance
 *  can be tried. */
export const useStartMfaConfiguration = () => {
  const navigate = useNavigate();

  return useMutation({
    mutationFn: (instance: InstanceInfo) =>
      startMfaConfiguration(instance, { source: ConfigureFactorsSource.AddPage }),
    onSuccess: () => {
      navigate({
        to: '/full/configure-mfa',
      });
    },
    onError: reportMfaConfigStartError,
  });
};
