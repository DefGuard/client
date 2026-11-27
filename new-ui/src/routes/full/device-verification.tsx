import { createFileRoute, redirect } from '@tanstack/react-router';
import { z } from 'zod';
import { DeviceVerificationPage } from '../../pages/full/DeviceVerificationPage/DeviceVerificationPage';
import type { DeviceVerificationSession } from '../../pages/full/DeviceVerificationPage/types';
import { deviceVerificationStartErrorMessage } from '../../pages/full/DeviceVerificationPage/utils/deviceVerificationErrorMessage';
import { openDeviceBlockedModal } from '../../pages/full/DeviceVerificationPage/utils/openDeviceBlockedModal';
import { startVerification } from '../../pages/full/DeviceVerificationPage/utils/startVerification';
import { Snackbar } from '../../shared/providers/snackbar/snackbar';
import { api } from '../../shared/rust-api/api';
import { getInstancesQueryOptions } from '../../shared/rust-api/query';

const searchSchema = z.object({
  instanceId: z.number(),
});

export const Route = createFileRoute('/full/device-verification')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ instanceId: search.instanceId }),
  // The session and its first code exist before the page shows, so it never loads.
  loader: async ({ deps, context }): Promise<DeviceVerificationSession> => {
    const start = await startVerification(deps.instanceId);
    if (!start.ok) {
      if (start.error.type === 'device_blocked') {
        void context.queryClient.invalidateQueries({
          queryKey: getInstancesQueryOptions.queryKey,
        });
        openDeviceBlockedModal();
      } else {
        Snackbar.error(deviceVerificationStartErrorMessage(start.error));
      }
      throw redirect({ to: '/full/overview', replace: true });
    }
    return { info: start.info, fetchedAt: start.fetchedAt };
  },
  // Covers Close, outcomes and any other way out of the page.
  onLeave: ({ search }) => {
    void api.cancelDeviceVerification(search.instanceId);
  },
  component: DeviceVerificationPage,
});
