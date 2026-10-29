import { useMutation } from '@tanstack/react-query';
import { Snackbar } from '../providers/snackbar/snackbar';
import { api } from '../rust-api/api';

export const useUpdateLocationRouting = (locationName: string) =>
  useMutation({
    mutationFn: api.updateLocationRouting,
    meta: {
      invalidate: ['locations'],
    },
    onSuccess: (autoConnectDisabled) => {
      if (autoConnectDisabled) {
        Snackbar.default(`Auto-connect turned off for ${locationName}`);
      }
    },
  });
