import { createFileRoute, Outlet } from '@tanstack/react-router';
import { FullPageLayout } from '../../shared/layouts/FullPageLayout/FullPageLayout';

export const Route = createFileRoute('/full/_default')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <FullPageLayout>
      <Outlet />
    </FullPageLayout>
  );
}
