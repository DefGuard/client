import { createFileRoute } from '@tanstack/react-router';
import { CompactEmptyPage } from '../pages/compact/CompactEmptyPage/CompactEmptyPage';

export const Route = createFileRoute('/empty')({
  component: CompactEmptyPage,
});
