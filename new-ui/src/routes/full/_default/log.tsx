import { createFileRoute } from '@tanstack/react-router';
import { LogPage } from '../../../pages/full/LogPage/LogPage';

export const Route = createFileRoute('/full/_default/log')({
  component: LogPage,
});
