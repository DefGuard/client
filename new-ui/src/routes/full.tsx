import { createFileRoute, Outlet } from '@tanstack/react-router';

export const Route = createFileRoute('/full')({
  component: RouteComponent,
});

function RouteComponent() {
  return <Outlet />;
}
