import { createFileRoute } from '@tanstack/react-router';
import { SettingsPage } from '../../../pages/full/SettingsPage/SettingsPage';

export const Route = createFileRoute('/full/_default/settings')({
  component: SettingsPage,
});
