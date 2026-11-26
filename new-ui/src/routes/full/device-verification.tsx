import { createFileRoute, redirect } from '@tanstack/react-router';
import { z } from 'zod';
import { DeviceVerificationPage } from '../../pages/full/DeviceVerificationPage/DeviceVerificationPage';
import {
  DEVICE_BLOCKED_MESSAGE,
  startVerification,
} from '../../pages/full/DeviceVerificationPage/startVerification';
import { Snackbar } from '../../shared/providers/snackbar/snackbar';
import { api } from '../../shared/rust-api/api';

const searchSchema = z.object({
  instanceId: z.number(),
});

export const Route = createFileRoute('/full/device-verification')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ instanceId: search.instanceId }),
  // The session and its first code exist before the page shows, so it never loads.
  loader: async ({ deps }) => {
    const start = await startVerification(deps.instanceId);
    if (!start.ok && start.error.type === 'device_blocked') {
      Snackbar.error(DEVICE_BLOCKED_MESSAGE);
      throw redirect({ to: '/full/overview', replace: true });
    }
    return start;
  },
  // Covers Close, outcomes and any other way out of the page.
  onLeave: ({ search }) => {
    void api.cancelDeviceVerification(search.instanceId);
  },
  component: DeviceVerificationPage,
});
