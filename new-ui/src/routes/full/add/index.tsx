import { createFileRoute } from '@tanstack/react-router';
import { AddPage } from '../../../pages/full/AddPage/AddPage';

export const Route = createFileRoute('/full/add/')({
  component: AddPage,
});
