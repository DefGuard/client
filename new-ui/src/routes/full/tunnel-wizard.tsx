import { createFileRoute } from '@tanstack/react-router';
import { TunnelWizardPage } from '../../pages/full/TunnelWizardPage/TunnelWizardPage';

export const Route = createFileRoute('/full/tunnel-wizard')({
  component: TunnelWizardPage,
});
