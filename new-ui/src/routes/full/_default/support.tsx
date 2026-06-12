import { createFileRoute } from '@tanstack/react-router';
import { SupportPage } from '../../../pages/full/SupportPage/SupportPage';

export const Route = createFileRoute('/full/_default/support')({
  component: SupportPage,
});
