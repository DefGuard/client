import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/empty')({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello "/empty"!</div>;
}
