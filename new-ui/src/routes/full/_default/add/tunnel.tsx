import { createFileRoute } from '@tanstack/react-router';
import { AddTunnelPage } from '../../../../pages/full/AddTunnelPage/AddTunnelPage';

export const Route = createFileRoute('/full/_default/add/tunnel')({
  component: AddTunnelPage,
});
