import { createFileRoute, redirect } from '@tanstack/react-router';
import { TunnelWizardPage } from '../../pages/full/TunnelWizardPage/TunnelWizardPage';
import { getInstancesQueryOptions, tunnelsDisabled } from '../../shared/rust-api/query';

export const Route = createFileRoute('/full/tunnel-wizard')({
  beforeLoad: async ({ context }) => {
    const instances = await context.queryClient.ensureQueryData(getInstancesQueryOptions);
    if (tunnelsDisabled(instances)) {
      throw redirect({ to: '/full/overview' });
    }
  },
  component: TunnelWizardPage,
});
