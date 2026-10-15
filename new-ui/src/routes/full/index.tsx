import { createFileRoute, Navigate, redirect } from '@tanstack/react-router';
import { error as logError } from '@tauri-apps/plugin-log';
import { api } from '../../shared/rust-api/api';
import {
  getInstancesQueryOptions,
  getProvisioningConfigQueryOptions,
  getTunnelsQueryOptions,
} from '../../shared/rust-api/query';
import { resolveFullIndexRedirect } from './-utils/resolveFullIndexRedirect';

export const Route = createFileRoute('/full/')({
  beforeLoad: async ({ context }) => {
    const pendingAddInstance = await api.takePendingAddInstance().catch(() => {
      void logError('Failed to fetch pending add-instance deep link.');
      return null;
    });
    if (pendingAddInstance) {
      throw redirect(resolveFullIndexRedirect({ pendingAddInstance }));
    }

    const [instances, tunnels, provisioningConfig] = await Promise.all([
      context.queryClient.fetchQuery(getInstancesQueryOptions),
      context.queryClient.fetchQuery(getTunnelsQueryOptions),
      context.queryClient
        .fetchQuery({ ...getProvisioningConfigQueryOptions, staleTime: 0 })
        .catch(() => {
          void logError('Failed to fetch provisioning config.');
          return null;
        }),
    ]);

    throw redirect(
      resolveFullIndexRedirect({
        instanceCount: instances.length,
        tunnelCount: tunnels.length,
        provisioningConfig,
        pendingAddInstance: null,
      }),
    );
  },
  component: RouteComponent,
});

function RouteComponent() {
  return <Navigate to="/full/overview" />;
}
