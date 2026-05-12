import { createFileRoute } from '@tanstack/react-router';
import { CompactLocationsPage } from '../pages/compact/CompactLocationsPage/CompactLocationsPage';
import { useCompactLocationStore } from '../pages/compact/CompactLocationsPage/hooks/useCompactLocationsStore';
import {
  getInstancesQueryOptions,
  getLocationsQueryOptions,
} from '../shared/rust-api/query';

export const Route = createFileRoute('/')({
  loader: async ({ context }) => {
    const instances = await context.queryClient.fetchQuery(getInstancesQueryOptions);
    if (instances.length > 0) {
      let selected = useCompactLocationStore.getState().selectedInstance;
      if (selected === null || instances.find((i) => i.id === selected) === null) {
        selected = instances[0].id;
      }
      const locations = await context.queryClient.fetchQuery(
        getLocationsQueryOptions(selected),
      );
      return {
        instances,
        locations,
      };
    }
    return {
      instances,
      locations: [],
    };
  },
  component: CompactLocationsPage,
});
