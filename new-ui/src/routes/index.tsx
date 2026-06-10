import { createFileRoute, redirect } from '@tanstack/react-router';
import { CompactLocationsPage } from '../pages/compact/CompactLocationsPage/CompactLocationsPage';
import {
  getInstancesQueryOptions,
  getLocationsQueryOptions,
  getTunnelsQueryOptions,
} from '../shared/rust-api/query';
import type { LocationInfo } from '../shared/rust-api/types';
import { useAppStore } from '../shared/store/useAppStore';

export const Route = createFileRoute('/')({
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

    let selected: NonNullable<typeof stored>;
    if (storedIsValid && stored !== null) {
      selected = stored;
    } else if (instances.length > 0) {
      selected = { kind: 'instance', data: instances[0] };
    } else {
      selected = { kind: 'tunnel', data: tunnels[0] };
    }

    if (!storedIsValid) {
      useAppStore.setState({ compactViewSelection: selected });
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
