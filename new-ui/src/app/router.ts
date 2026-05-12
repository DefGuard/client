import { createRouter } from '@tanstack/react-router';
import { routeTree } from '../routeTree.gen';
import { NotFoundRoute } from '../shared/components/NotFoundRoute/NotFoundRoute';
import { queryClient } from './query';

export const router = createRouter({
  routeTree,
  defaultPreloadStaleTime: 0,
  defaultNotFoundComponent: NotFoundRoute,
  context: {
    queryClient,
  },
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
