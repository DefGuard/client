import { useNavigate } from '@tanstack/react-router';
import { type PropsWithChildren, useEffect } from 'react';
import { useEnrollmentStore } from '../hooks/useEnrollmentStore';

export const EnrollmentTimeoutProvider = ({ children }: PropsWithChildren) => {
  const navigate = useNavigate();
  const deadline = useEnrollmentStore((s) => s.deadline);

  useEffect(() => {
    if (!deadline) return;

    const ms = new Date(deadline).getTime() - Date.now();
    if (ms <= 0) {
      void navigate({ to: '/full/session-timeout' });
      return;
    }

    const timer = setTimeout(() => {
      void navigate({ to: '/full/session-timeout' });
    }, ms);

    return () => clearTimeout(timer);
  }, [deadline, navigate]);

  return children;
};
