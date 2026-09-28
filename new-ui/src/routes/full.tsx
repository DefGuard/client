import { createFileRoute, Outlet } from '@tanstack/react-router';
import { EdgeComsErrorHost } from '../pages/full/ConfigureMfaPage/components/EdgeComsError/EdgeComsError';

export const Route = createFileRoute('/full')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <EdgeComsErrorHost>
      <Outlet />
    </EdgeComsErrorHost>
  );
}
