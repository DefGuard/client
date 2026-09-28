import { error as logError } from '@tauri-apps/plugin-log';
import { Snackbar } from '../../../../shared/providers/snackbar/snackbar';
import {
  isMfaConfigMissingToken,
  isMfaConfigNetworkError,
  isMfaConfigUnsupported,
} from '../../../../shared/rust-api/mfaError';
import { showEdgeComsError } from '../components/EdgeComsError/useEdgeComsErrorStore';

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
  if (isMfaConfigNetworkError(err)) {
    showEdgeComsError();
    return;
  }
  Snackbar.error('Could not start MFA configuration.');
};
