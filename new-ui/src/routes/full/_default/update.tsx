import { createFileRoute } from '@tanstack/react-router';
import { UpdatePage } from '../../../pages/full/UpdatePage/UpdatePage';

export const Route = createFileRoute('/full/_default/update')({
  component: UpdatePage,
});
