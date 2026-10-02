import {
  createContext,
  type PropsWithChildren,
  useCallback,
  useContext,
  useEffect,
  useState,
} from 'react';
import { useConfigureMfaStore } from '../hooks/useConfigureMfaStore';
import { ConfigureMfaStep } from '../types';
import { SessionTimeoutPage } from './SessionTimeoutPage/SessionTimeoutPage';

const ConfigureMfaTimeoutContext = createContext<(() => void) | null>(null);

/** Marks the session expired, for when the proxy reports it before the local timer fires. */
export const useConfigureMfaSessionExpired = (): (() => void) => {
  const expire = useContext(ConfigureMfaTimeoutContext);
  if (!expire) {
    throw new Error(
      'useConfigureMfaSessionExpired must be used within ConfigureMfaTimeoutProvider',
    );
  }
  return expire;
};

/** Recover at the deadline rather than let the user type a code that cannot land. */
export const ConfigureMfaTimeoutProvider = ({ children }: PropsWithChildren) => {
  const [expired, setExpired] = useState(false);
  const deadline = useConfigureMfaStore((s) => s.deadline);
  // Picking only the email fallback opens on Finish before verification, hence the auth check.
  const settled = useConfigureMfaStore(
    (s) =>
      s.authorized &&
      (s.activeStep === ConfigureMfaStep.RecoveryCodes ||
        s.activeStep === ConfigureMfaStep.Finish),
  );

  const expire = useCallback(() => setExpired(true), []);

  useEffect(() => {
    if (!deadline || settled) return;

    const ms = new Date(deadline).getTime() - Date.now();
    if (ms <= 0) {
      expire();
      return;
    }

    const timer = setTimeout(expire, ms);
    return () => clearTimeout(timer);
  }, [deadline, settled, expire]);

  return (
    <ConfigureMfaTimeoutContext.Provider value={expire}>
      {expired ? <SessionTimeoutPage /> : children}
    </ConfigureMfaTimeoutContext.Provider>
  );
};
