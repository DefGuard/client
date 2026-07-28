import { createFileRoute, redirect } from '@tanstack/react-router';
import { TunnelWizardPage } from '../../pages/full/TunnelWizardPage/TunnelWizardPage';
import { getInstancesQueryOptions } from '../../shared/rust-api/query';

export const Route = createFileRoute('/full/tunnel-wizard')({
  beforeLoad: async ({ context }) => {
    const instances = await context.queryClient.ensureQueryData(getInstancesQueryOptions);
    if (instances.some((i) => i.disable_tunnels)) {
      throw redirect({ to: '/full/overview' });
    }
  },
  component: TunnelWizardPage,
});
