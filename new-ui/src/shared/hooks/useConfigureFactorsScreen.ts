import { useMutation } from '@tanstack/react-query';
import { error as logError } from '@tauri-apps/plugin-log';
import { Snackbar } from '../providers/snackbar/snackbar';
import { api } from '../rust-api/api';

/** Backend opens the Configure MFA wizard in the full view from either window. */
export const useConfigureFactorsScreen = (options?: { onSuccess?: () => void }) =>
  useMutation({
    mutationFn: api.initiateConfigureFactorScreen,
    onSuccess: options?.onSuccess,
    onError: (err) => {
      void logError(`Failed to open the MFA configuration screen: ${err}`);
      Snackbar.error('Could not open MFA configuration.');
    },
  });
