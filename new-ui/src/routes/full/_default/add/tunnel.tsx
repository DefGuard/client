import { createFileRoute, redirect } from '@tanstack/react-router';
import { AddTunnelPage } from '../../../../pages/full/AddTunnelPage/AddTunnelPage';
import {
  getInstancesQueryOptions,
  tunnelsDisabled,
} from '../../../../shared/rust-api/query';

export const Route = createFileRoute('/full/_default/add/tunnel')({
  beforeLoad: async ({ context }) => {
    const instances = await context.queryClient.ensureQueryData(getInstancesQueryOptions);
    if (tunnelsDisabled(instances)) {
      throw redirect({ to: '/full/overview' });
    }
  },
  component: AddTunnelPage,
});
