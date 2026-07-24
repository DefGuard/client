import { useNavigate } from '@tanstack/react-router';
import { useCallback } from 'react';
import { parseEnrollmentError } from '../../../../shared/rust-api/enrollment';
import { EnrollmentErrorCopy } from '../errorCopy';
import { useEnrollmentErrorModal } from './useEnrollmentErrorModal';

export const useEnrollmentErrorHandler = () => {
  const navigate = useNavigate();

  const handleError = useCallback(
    (err: unknown, copy: { title: string; message: string }) => {
      const parsed = parseEnrollmentError(err);
      if (parsed.errorKind === 'unauthorized') {
        void navigate({ to: '/full/session-timeout' });
        return;
      }
      if (parsed.errorKind === 'network') {
        useEnrollmentErrorModal.getState().open(EnrollmentErrorCopy.generic);
        return;
      }
      useEnrollmentErrorModal.getState().open(copy);
    },
    [navigate],
  );

  return handleError;
};
