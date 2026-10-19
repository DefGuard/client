import { createFileRoute, redirect } from '@tanstack/react-router';
import { SelectMfaInstancePage } from '../../../../pages/full/SelectMfaInstancePage/SelectMfaInstancePage';
import { getInstancesQueryOptions } from '../../../../shared/rust-api/query';
import { mfaConfigurableInstances } from '../../../../shared/utils/mfa';

export const Route = createFileRoute('/full/_default/add/mfa')({
  beforeLoad: async ({ context }) => {
    const instances = await context.queryClient.ensureQueryData(getInstancesQueryOptions);
    if (mfaConfigurableInstances(instances).length === 0) {
      throw redirect({ to: '/full/add' });
    }
  },
  component: SelectMfaInstancePage,
});
