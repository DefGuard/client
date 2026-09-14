import { createFileRoute, redirect } from '@tanstack/react-router';
import { SelectMfaInstancePage } from '../../../../pages/full/SelectMfaInstancePage/SelectMfaInstancePage';
import {
  getInstancesQueryOptions,
  mfaConfigurableInstances,
} from '../../../../shared/rust-api/query';

export const Route = createFileRoute('/full/_default/add/mfa')({
  beforeLoad: async ({ context }) => {
    const instances = await context.queryClient.ensureQueryData(getInstancesQueryOptions);
    // Nothing to pick between, the Add page starts the flow itself.
    if (mfaConfigurableInstances(instances).length < 2) {
      throw redirect({ to: '/full/add' });
    }
  },
  component: SelectMfaInstancePage,
});
