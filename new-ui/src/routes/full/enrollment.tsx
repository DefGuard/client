import { createFileRoute } from '@tanstack/react-router';
import { EnrollmentPage } from '../../pages/full/EnrollmentPage/EnrollmentPage';

export const Route = createFileRoute('/full/enrollment')({
  component: EnrollmentPage,
});
