import { createFileRoute, Outlet } from '@tanstack/react-router';
import { AppDataProvider } from '../shared/providers/AppDataContext';

export const Route = createFileRoute('/full')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <AppDataProvider>
      <Outlet />
    </AppDataProvider>
  );
}
