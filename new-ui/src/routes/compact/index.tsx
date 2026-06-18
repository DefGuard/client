import { createFileRoute, redirect } from '@tanstack/react-router';
import { CompactLocationsPage } from '../../pages/compact/CompactLocationsPage/CompactLocationsPage';
import { api } from '../../shared/rust-api/api';
import {
  getInstancesQueryOptions,
  getLocationsQueryOptions,
  getSessionStateQueryOptions,
  getTunnelsQueryOptions,
} from '../../shared/rust-api/query';
import type { LocationInfo, OverviewViewSelection } from '../../shared/rust-api/types';

export const Route = createFileRoute('/compact/')({
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
      storedIsValid = instances.some((i) => i.id === stored.data.id);
    } else {
      storedIsValid = tunnels.some((t) => t.id === stored.data.id);
    }

    let selected: OverviewViewSelection;
    if (storedIsValid && stored !== null) {
      selected = stored;
    } else if (instances.length > 0) {
      selected = { kind: 'instance', data: instances[0] };
    } else {
      selected = { kind: 'tunnel', data: tunnels[0] };
    }

    if (!storedIsValid) {
      await api.patchSessionState({ view_selection: selected });
      await context.queryClient.invalidateQueries({ queryKey: ['session-state'] });
    }

    let locations: LocationInfo[];
    if (selected.kind === 'instance') {
      locations = await context.queryClient.fetchQuery(
        getLocationsQueryOptions(selected.data.id),
      );
    } else {
      locations = [];
    }

    return { instances, tunnels, locations };
  },
  component: CompactLocationsPage,
});
