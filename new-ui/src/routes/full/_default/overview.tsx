import { createFileRoute, redirect } from '@tanstack/react-router';
import { OverviewPage } from '../../../pages/full/OverviewPage/OverviewPage';
import {
  getInstancesQueryOptions,
  getTunnelsQueryOptions,
} from '../../../shared/rust-api/query';
import { useAppStore } from '../../../shared/store/useAppStore';

export const Route = createFileRoute('/full/_default/overview')({
  loader: async ({ context }) => {
    const [instances, tunnels] = await Promise.all([
      context.queryClient.fetchQuery(getInstancesQueryOptions),
      context.queryClient.fetchQuery(getTunnelsQueryOptions),
    ]);

    if (instances.length === 0 && tunnels.length === 0) {
      throw redirect({ to: '/empty' });
    }

    const stored = useAppStore.getState().compactViewSelection;

    let storedIsValid: boolean;
    if (stored === null) {
      storedIsValid = false;
    } else if (stored.kind === 'instance') {
      storedIsValid = instances.some((i) => i.id === stored.data.id);
    } else {
      storedIsValid = tunnels.some((t) => t.id === stored.data.id);
    }

    if (!storedIsValid) {
      const selected =
        instances.length > 0
          ? { kind: 'instance' as const, data: instances[0] }
          : { kind: 'tunnel' as const, data: tunnels[0] };
      useAppStore.setState({ compactViewSelection: selected });
    }
  },
  component: OverviewPage,
});
