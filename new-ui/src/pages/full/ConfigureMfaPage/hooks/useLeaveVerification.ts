import { useMutation } from '@tanstack/react-query';
import { useCallback } from 'react';
import { showsVerificationPicker } from '../utils';
import { useConfigureMfaStore } from './useConfigureMfaStore';

/** Verification opens the flow, so leaving it ends the session unless a method picker came first. */
export const useLeaveVerification = (onCancel: () => Promise<void>) => {
  const hasPicker = useConfigureMfaStore((s) =>
    showsVerificationPicker(s.verificationMethods),
  );

  const { mutate: cancel, isPending: isLeaving } = useMutation({
    mutationFn: onCancel,
  });

  const leave = useCallback(() => {
    if (hasPicker) {
      useConfigureMfaStore.getState().backFromVerification();
      return;
    }
    cancel();
  }, [hasPicker, cancel]);

  return { leaveText: hasPicker ? 'Back' : 'Cancel', leave, isLeaving };
};
