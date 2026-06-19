import { createFileRoute, redirect } from '@tanstack/react-router';
import { OverviewPage } from '../../../pages/full/OverviewPage/OverviewPage';
import { api } from '../../../shared/rust-api/api';
import {
  getInstancesQueryOptions,
  getSessionStateQueryOptions,
  getTunnelsQueryOptions,
} from '../../../shared/rust-api/query';

export const Route = createFileRoute('/full/_default/overview')({
  loader: async ({ context }) => {
    const [instances, tunnels] = await Promise.all([
      context.queryClient.fetchQuery(getInstancesQueryOptions),
      context.queryClient.fetchQuery(getTunnelsQueryOptions),
    ]);

    if (instances.length === 0 && tunnels.length === 0) {
      throw redirect({ to: '/empty' });
    }

    const sessionState = await context.queryClient.fetchQuery(
      getSessionStateQueryOptions,
    );
    const stored = sessionState?.view_selection ?? null;

    let storedIsValid: boolean;
    if (stored === null) {
      storedIsValid = false;
    } else if (stored.kind === 'instance') {
      storedIsValid = instances.some((i) => i.id === stored.id);
    } else {
      storedIsValid = tunnels.some((t) => t.id === stored.id);
    }

    if (!storedIsValid) {
      const selected =
        instances.length > 0
          ? { kind: 'instance' as const, id: instances[0].id }
          : { kind: 'tunnel' as const, id: tunnels[0].id };
      await api.patchSessionState({ view_selection: selected });
      await context.queryClient.invalidateQueries({ queryKey: ['session-state'] });
    }
  },
  component: OverviewPage,
});
