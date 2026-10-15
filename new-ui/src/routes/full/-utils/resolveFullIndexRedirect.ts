import type {
  AddInstanceEventPayload,
  ProvisioningConfig,
} from '../../../shared/rust-api/types';

type ResolveFullIndexRedirectArgs =
  | {
      pendingAddInstance: AddInstanceEventPayload;
      instanceCount?: number;
      tunnelCount?: number;
      provisioningConfig?: ProvisioningConfig | null;
    }
  | {
      instanceCount: number;
      tunnelCount: number;
      provisioningConfig: ProvisioningConfig | null;
      pendingAddInstance: null;
    };

type FullIndexRedirect =
  | { to: '/full/add/instance'; search: { url: string; token: string } }
  | { to: '/full/add' }
  | { to: '/full/overview' };

export const resolveFullIndexRedirect = (
  args: ResolveFullIndexRedirectArgs,
): FullIndexRedirect => {
  if (args.pendingAddInstance) {
    return {
      to: '/full/add/instance',
      search: {
        url: args.pendingAddInstance.url,
        token: args.pendingAddInstance.token,
      },
    };
  }
  const { instanceCount, tunnelCount, provisioningConfig } = args;
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
