import type { DeviceVerificationError } from '../../../../shared/rust-api/types';

export const deviceBlockedMessage = 'This device has been blocked by your administrator.';

export const deviceVerificationErrorMessage = (
  error: DeviceVerificationError,
): string => {
  switch (error.type) {
    case 'network':
      return 'Could not reach the server. Check your connection and try again.';
    case 'license_required':
      return 'Device verification requires an enterprise license.';
    case 'device_token_invalid':
      return 'This device is no longer registered with this instance. Add the instance again.';
    case 'user_inactive':
      return 'Your account is inactive. Contact your administrator.';
    case 'device_blocked':
      return deviceBlockedMessage;
    case 'session_not_found':
      return 'The verification session has ended. Start the verification again.';
    case 'unsupported_algorithm':
      return 'The server uses a verification method this client does not support. Update the client.';
    case 'cancelled':
      return 'Verification was cancelled.';
    case 'invalid_response':
    case 'proxy':
    case 'other':
      return `Device verification failed: ${error.message}`;
  }
};
