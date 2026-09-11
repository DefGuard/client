import { createFileRoute } from '@tanstack/react-router';
import { SessionTimeoutPage } from '../../pages/full/SessionTimeoutPage/SessionTimeoutPage';

export const Route = createFileRoute('/full/session-timeout')({
  component: SessionTimeoutPage,
});
