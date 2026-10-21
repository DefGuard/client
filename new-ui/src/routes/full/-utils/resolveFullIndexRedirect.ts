import type { ProvisioningConfig } from '../../../shared/rust-api/types';

type ResolveFullIndexRedirectArgs = {
  instanceCount: number;
  tunnelCount: number;
  provisioningConfig: ProvisioningConfig | null;
};

type FullIndexRedirect =
  | { to: '/full/add/instance'; search: { url: string; token: string } }
  | { to: '/full/add' }
  | { to: '/full/overview' };

export const resolveFullIndexRedirect = ({
  instanceCount,
  tunnelCount,
  provisioningConfig,
}: ResolveFullIndexRedirectArgs): FullIndexRedirect => {
  if (instanceCount === 0 && provisioningConfig) {
    return {
      to: '/full/add/instance',
      search: {
        url: provisioningConfig.enrollment_url,
        token: provisioningConfig.enrollment_token,
      },
    };
  }
  if (instanceCount === 0 && tunnelCount === 0) {
    return { to: '/full/add' };
  }
  return { to: '/full/overview' };
};
