import { useQuery } from '@tanstack/react-query';
import { error } from '@tauri-apps/plugin-log';
import { useEffect } from 'react';
import { clientApi } from '../pages/client/clientAPI/clientApi';
import type { ProvisioningConfig } from '../pages/client/clientAPI/types';
import { clientQueryKeys } from '../pages/client/query';
import { useToaster } from '../shared/defguard-ui/hooks/toasts/useToaster';
import useAddInstance from '../shared/hooks/useAddInstance';

const { getProvisioningConfig } = clientApi;

export default function AutoProvisioningManager() {
  const toaster = useToaster();
  const { handleAddInstance } = useAddInstance();
  const { data: provisioningConfig } = useQuery({
    queryFn: getProvisioningConfig,
    queryKey: [clientQueryKeys.getProvisioningConfig],
    refetchOnMount: false,
    refetchOnWindowFocus: false,
  });

  const handleProvisioning = async (config: ProvisioningConfig) => {
    try {
      await handleAddInstance({
        url: config.enrollment_url,
        token: config.enrollment_token,
      });
    } catch (e) {
      error(
        `Failed to handle automatic client provisioning with ${JSON.stringify(config)}.\n Error: ${JSON.stringify(e)}`,
      );
      toaster.error(
        'Automatic client provisioning failed, please contact your administrator.',
      );
    }
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: migration, checkMeLater
  useEffect(() => {
    if (provisioningConfig) {
      handleProvisioning(provisioningConfig);
    }
  }, [provisioningConfig]);

  return null;
}
