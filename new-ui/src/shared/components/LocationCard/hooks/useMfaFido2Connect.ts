import { error } from '@tauri-apps/plugin-log';
import { useCallback, useState } from 'react';
import { api } from '../../../rust-api/api';
import { isConnectFailure, mfaErrorMessage } from '../../../rust-api/mfaError';
import type { LocationInfo } from '../../../rust-api/types';

type UseMfaFido2ConnectOptions = {
  onConnected?: () => void;
};

/**
 * FIDO2 MFA: the user types the security key PIN and the client hands it to the
 * backend, which verifies it and brings the connection up in one step. The
 * backend is a stub for now, so this opens no proxy MFA session - hence no
 * `/start` call and no token to carry here.
 */
export const useMfaFido2Connect = (
  location: LocationInfo,
  { onConnected }: UseMfaFido2ConnectOptions = {},
) => {
  const [isVerifying, setIsVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState<string | null>(null);

  const verifyPin = useCallback(
    async (pin: string) => {
      setIsVerifying(true);
      setVerifyError(null);

      try {
        // Completes MFA and brings up the connection in the backend, the same
        // way the code-based methods do.
        await api.mfaFido2Pin(location.instance_id, location.id, pin);
        onConnected?.();
      } catch (err) {
        void error(`FIDO2 MFA failed: ${err}`);
        const message = mfaErrorMessage(err);
        setVerifyError(
          isConnectFailure(message)
            ? 'Failed to establish VPN connection'
            : 'Verification failed',
        );
      } finally {
        setIsVerifying(false);
      }
    },
    [location.instance_id, location.id, onConnected],
  );

  return { verifyPin, isVerifying, verifyError };
};
