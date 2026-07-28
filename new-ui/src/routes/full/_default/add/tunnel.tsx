import { createFileRoute, redirect } from '@tanstack/react-router';
import { AddTunnelPage } from '../../../../pages/full/AddTunnelPage/AddTunnelPage';
import { getInstancesQueryOptions } from '../../../../shared/rust-api/query';

export const Route = createFileRoute('/full/_default/add/tunnel')({
  beforeLoad: async ({ context }) => {
    const instances = await context.queryClient.ensureQueryData(getInstancesQueryOptions);
    if (instances.some((i) => i.disable_tunnels)) {
      throw redirect({ to: '/full/overview' });
    }
  },
  component: AddTunnelPage,
});
