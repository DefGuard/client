import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { debug } from 'tauri-plugin-log-api';

import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { LoaderSpinner } from '../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { routes } from '../../shared/routes';
import { clientApi } from '../client/clientAPI/clientApi';
import { useClientStore } from '../client/hooks/useClientStore';
import { clientQueryKeys } from '../client/query';
import { ApplicationUnlockModal } from './components/ApplicationUnlockModal/ApplicationUnlockModal';
import { useApplicationUnlockModal } from './components/ApplicationUnlockModal/useApplicationUnlockModal';

const { getDatabaseConnectionStatus, getSettings, getInstances, getTunnels } = clientApi;

// TODO: Make it defguard logo instead of spinner or smth...
export const SplashPage = () => {
  const navigate = useNavigate();
  const openUnlockModal = useApplicationUnlockModal((s) => s.open);
  const setClientState = useClientStore((state) => state.setState);

  const { data: connectionStatus } = useQuery({
    queryFn: getDatabaseConnectionStatus,
    queryKey: [clientQueryKeys.getAppDatabaseConnectionStatus],
  });

  // check if database has connection
  useEffect(() => {
    if (connectionStatus !== undefined) {
      if (connectionStatus) {
        const loadTauriState = async () => {
          debug('App init state from tauri');
          const settings = await getSettings();
          const instances = await getInstances();
          const tunnels = await getTunnels();
          setClientState({ settings, instances, tunnels });
          debug('Tauri init data loaded');
          navigate(routes.client.base, { replace: true });
        };
        loadTauriState();
      } else {
        openUnlockModal();
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connectionStatus]);

  return (
    <PageContainer id="splash-page">
      <LoaderSpinner size={128} />
      <ApplicationUnlockModal />
    </PageContainer>
  );
};
