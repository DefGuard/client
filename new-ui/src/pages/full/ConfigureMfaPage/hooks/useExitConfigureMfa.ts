import { useNavigate } from '@tanstack/react-router';
import { useCallback } from 'react';
import { useAppData } from '../../../../shared/providers/AppDataContext';
import { isPresent } from '../../../../shared/utils/isPresent';
import { discardMfaConfiguration, useConfigureMfaStore } from './useConfigureMfaStore';

/** Returns to where the flow was opened, then ends the session. The discard resets the store,
 *  so the page has to be gone first or it re-renders on an empty one. */
export const useExitConfigureMfa = (): (() => Promise<void>) => {
  const navigate = useNavigate();
  const { setViewSelection } = useAppData();

  return useCallback(async () => {
    const { location } = useConfigureMfaStore.getState();
    if (isPresent(location)) {
      setViewSelection({ kind: 'instance', id: location.instance_id });
      await navigate({ to: '/full/overview', replace: true });
    } else {
      await navigate({ to: '/full/add', replace: true });
    }
    await discardMfaConfiguration();
  }, [navigate, setViewSelection]);
};
