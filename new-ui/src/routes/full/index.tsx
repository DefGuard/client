import { createFileRoute, Navigate, redirect } from '@tanstack/react-router';
import {
  getInstancesQueryOptions,
  getTunnelsQueryOptions,
} from '../../shared/rust-api/query';

export const Route = createFileRoute('/full/')({
  beforeLoad: async ({ context }) => {
    const [instances, tunnels] = await Promise.all([
      context.queryClient.fetchQuery(getInstancesQueryOptions),
      context.queryClient.fetchQuery(getTunnelsQueryOptions),
    ]);

    if (instances.length === 0 && tunnels.length === 0) {
      throw redirect({ to: '/full/add' });
    } else {
      throw redirect({ to: '/full/overview' });
    }
  },
  component: RouteComponent,
});

function RouteComponent() {
  return <Navigate to="/full/overview" />;
}
