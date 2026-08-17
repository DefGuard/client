import { error } from '@tauri-apps/plugin-log';
import { useCallback, useState } from 'react';
import { api } from '../../../rust-api/api';
import { mfaErrorMessage } from '../../../rust-api/mfaError';
import type { LocationInfo } from '../../../rust-api/types';

/**
 * FIDO2 MFA flow: the user types the security key PIN and it is handed over to
 * the backend. The backend command is a stub for now - it acknowledges the PIN
 * and returns a status message instead of bringing the connection up, so there
 * is no `/start` call and no token to keep track of here.
 */
export const useMfaFido2Connect = (location: LocationInfo) => {
  const [isVerifying, setIsVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const verifyPin = useCallback(
    async (pin: string) => {
      setIsVerifying(true);
      setVerifyError(null);
      setMessage(null);

      try {
        const result = await api.mfaFido2Pin(location.instance_id, location.id, pin);
        setMessage(result);
      } catch (err) {
        void error(`FIDO2 MFA failed: ${err}`);
        setVerifyError(mfaErrorMessage(err));
      } finally {
        setIsVerifying(false);
      }
    },
    [location.instance_id, location.id],
  );

  return { verifyPin, isVerifying, verifyError, message };
};
