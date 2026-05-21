import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';
import { TauriEventProvider } from '../shared/providers/TauriEventProvider';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
  pendingMs: 500,
  pendingMinMs: 250,
});

function RootComponent() {
  return (
    <TauriEventProvider>
      <Outlet />
    </TauriEventProvider>
  );
}
