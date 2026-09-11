import { createFileRoute } from '@tanstack/react-router';
import { WelcomePage } from '../pages/welcome/WelcomePage';

export const Route = createFileRoute('/welcome')({
  component: WelcomePage,
});
