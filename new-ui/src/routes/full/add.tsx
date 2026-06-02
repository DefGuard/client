import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/full/add')({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello "/full/add"!</div>;
}
