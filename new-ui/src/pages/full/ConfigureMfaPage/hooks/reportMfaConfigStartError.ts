import { error as logError } from '@tauri-apps/plugin-log';
import { Snackbar } from '../../../../shared/providers/snackbar/snackbar';
import {
  isMfaConfigMissingToken,
  isMfaConfigUnsupported,
} from '../../../../shared/rust-api/mfaError';

/** How a failed `startMfaConfiguration` is reported, shared by every entry point. */
export const reportMfaConfigStartError = (err: unknown): void => {
  void logError(`MFA configuration start failed: ${err}`);
  if (isMfaConfigUnsupported(err)) {
    Snackbar.error(
      'This Defguard instance does not support configuring MFA from the client.',
    );
    return;
  }
  if (isMfaConfigMissingToken(err)) {
    Snackbar.error(
      'This device has no polling token; update the instance and try again.',
    );
    return;
  }
  Snackbar.error('Could not start MFA configuration.');
};
