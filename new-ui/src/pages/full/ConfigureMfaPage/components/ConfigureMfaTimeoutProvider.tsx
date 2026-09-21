import { useNavigate } from '@tanstack/react-router';
import { type PropsWithChildren, useEffect } from 'react';
import { Snackbar } from '../../../../shared/providers/snackbar/snackbar';
import {
  discardMfaConfiguration,
  useConfigureMfaStore,
} from '../hooks/useConfigureMfaStore';

/** Recover at the deadline rather than let the user type a code that cannot land. */
export const ConfigureMfaTimeoutProvider = ({ children }: PropsWithChildren) => {
  const navigate = useNavigate();
  const deadline = useConfigureMfaStore((s) => s.deadline);

  useEffect(() => {
    if (!deadline) return;

    const expire = () => {
      void discardMfaConfiguration();
      Snackbar.error('MFA configuration session expired, start again.');
      void navigate({ to: '/full/add', replace: true });
    };

    const ms = new Date(deadline).getTime() - Date.now();
    if (ms <= 0) {
      expire();
      return;
    }

    const timer = setTimeout(expire, ms);
    return () => clearTimeout(timer);
  }, [deadline, navigate]);

  return children;
};
