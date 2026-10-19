import { error as logError } from '@tauri-apps/plugin-log';
import { Snackbar } from '../../../../shared/providers/snackbar/snackbar';
import {
  isMfaConfigMfaNotUsable,
  isMfaConfigMissingToken,
  isMfaConfigNetworkError,
  isMfaConfigUnsupported,
} from '../../../../shared/rust-api/mfaError';
import { showEdgeComsError } from '../components/EdgeComsError/useEdgeComsErrorStore';
import { NoVerificationMethodError } from './noVerificationMethodError';

/** How a failed `startMfaConfiguration` is reported, shared by every entry point. */
export const reportMfaConfigStartError = (err: unknown): void => {
  void logError(`MFA configuration start failed: ${err}`);
  if (err instanceof NoVerificationMethodError) {
    Snackbar.error(
      'There is no method available to verify your identity on this Defguard instance. Contact your administrator.',
    );
    return;
  }
  if (isMfaConfigMfaNotUsable(err)) {
    Snackbar.error(
      "Your account's MFA, such as a passkey added in the browser, can't be used in the desktop client. Add an authenticator app, email codes or a certified security key in your Defguard profile.",
    );
    return;
  }
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
