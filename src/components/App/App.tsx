import 'dayjs/locale/en';
import '../../shared/defguard-ui/scss/index.scss';
import '../../shared/scss/index.scss';

import { QueryClient } from '@tanstack/query-core';
import { QueryClientProvider } from '@tanstack/react-query';
import dayjs from 'dayjs';
import customParseData from 'dayjs/plugin/customParseFormat';
import duration from 'dayjs/plugin/duration';
import localeData from 'dayjs/plugin/localeData';
import relativeTime from 'dayjs/plugin/relativeTime';
import timezone from 'dayjs/plugin/timezone';
import updateLocale from 'dayjs/plugin/updateLocale';
import utc from 'dayjs/plugin/utc';
import { useEffect, useMemo, useRef, useState } from 'react';
import { createBrowserRouter, Navigate, RouterProvider } from 'react-router-dom';
import { debug } from 'tauri-plugin-log-api';
import { localStorageDetector } from 'typesafe-i18n/detectors';

import TypesafeI18n from '../../i18n/i18n-react';
import { detectLocale } from '../../i18n/i18n-util';
import { loadLocaleAsync } from '../../i18n/i18n-util.async';
import { clientApi } from '../../pages/client/clientAPI/clientApi';
import { ClientPage } from '../../pages/client/ClientPage';
import { useClientStore } from '../../pages/client/hooks/useClientStore';
import { CarouselPage } from '../../pages/client/pages/CarouselPage/CarouselPage';
import { ClientAddedPage } from '../../pages/client/pages/ClientAddedPage/ClientAddedPage';
import { ClientAddInstancePage } from '../../pages/client/pages/ClientAddInstancePage/ClientAddInstnacePage';
import { ClientAddTunnelPage } from '../../pages/client/pages/ClientAddTunnelPage/ClientAddTunnelPage';
import { ClientEditTunnelPage } from '../../pages/client/pages/ClientEditTunnelPage/ClientEditTunnelPage';
import { ClientInstancePage } from '../../pages/client/pages/ClientInstancePage/ClientInstancePage';
import { ClientSettingsPage } from '../../pages/client/pages/ClientSettingsPage/ClientSettingsPage';
import { WireguardInstanceType } from '../../pages/client/types';
import { EnrollmentPage } from '../../pages/enrollment/EnrollmentPage';
import { SessionTimeoutPage } from '../../pages/sessionTimeout/SessionTimeoutPage';
import { ToastManager } from '../../shared/defguard-ui/components/Layout/ToastManager/ToastManager';
import { useTheme } from '../../shared/defguard-ui/hooks/theme/useTheme';
import { ThemeProvider } from '../../shared/providers/ThemeProvider/ThemeProvider';
import { routes } from '../../shared/routes';
import { ApplicationUpdateManager } from '../ApplicationUpdateManager/ApplicationUpdateManager';

dayjs.extend(duration);
dayjs.extend(utc);
dayjs.extend(customParseData);
dayjs.extend(relativeTime);
dayjs.extend(localeData);
dayjs.extend(updateLocale);
dayjs.extend(timezone);

const queryClient = new QueryClient();

const { getAppConfig, getInstances, getTunnels } = clientApi;

const router = createBrowserRouter([
  {
    index: true,
    element: <Navigate to={routes.client.base} />,
  },
  {
    path: '/timeout',
    element: <SessionTimeoutPage />,
  },
  {
    path: '/enrollment',
    element: <EnrollmentPage />,
  },
  {
    path: '/client',
    element: <ClientPage />,
    children: [
      {
        path: '/client/',
        index: true,
        element: <Navigate to={routes.client.instancePage} />,
      },
      {
        path: '/client/instance',
        element: <ClientInstancePage />,
      },
      {
        path: '/client/carousel',
        element: <CarouselPage />,
      },
      {
        path: '/client/add-instance',
        element: <ClientAddInstancePage />,
      },
      {
        path: '/client/instance-created',
        element: <ClientAddedPage pageType={WireguardInstanceType.DEFGUARD_INSTANCE} />,
      },
      {
        path: '/client/add-tunnel',
        element: <ClientAddTunnelPage />,
      },
      {
        path: '/client/tunnel-created',
        element: <ClientAddedPage pageType={WireguardInstanceType.TUNNEL} />,
      },
      {
        path: '/client/edit-tunnel',
        element: <ClientEditTunnelPage />,
      },
      {
        path: '/client/settings',
        element: <ClientSettingsPage />,
      },
      {
        path: '/client/*',
        element: <Navigate to={routes.client.base} />,
      },
    ],
  },
  {
    path: '/*',
    element: <Navigate to={routes.client.base} replace />,
  },
]);

const detectedLocale = detectLocale(localStorageDetector);

export const App = () => {
  //workaround ensure effect once in dev mode, thanks react :3
  const tauriInitLoadRef = useRef(false);
  const localeLoadRef = useRef(false);
  const [localeLoaded, setWasLoaded] = useState(false);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const setClientState = useClientStore((state) => state.setState);
  const { changeTheme } = useTheme();

  const appLoaded = useMemo(
    () => localeLoaded && settingsLoaded,
    [localeLoaded, settingsLoaded],
  );

  // load locales
  useEffect(() => {
    if (!localeLoadRef.current) {
      localeLoadRef.current = true;
      debug('Loading locales');
      loadLocaleAsync(detectedLocale).then(() => {
        setWasLoaded(true);
        debug(`Locale ${detectedLocale} loaded.`);
      });
      dayjs.locale(detectedLocale);
    }
  }, []);

  // load settings from tauri first time
  useEffect(() => {
    if (!tauriInitLoadRef.current) {
      tauriInitLoadRef.current = true;
      const loadTauriState = async () => {
        debug('App init state from tauri');
        const appConfig = await getAppConfig();
        const instances = await getInstances();
        const tunnels = await getTunnels();
        changeTheme(appConfig.theme);
        setClientState({ appConfig, instances, tunnels });
        debug('Tauri init data loaded');
        setSettingsLoaded(true);
      };
      loadTauriState();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!appLoaded) return null;

  return (
    <TypesafeI18n locale={detectedLocale}>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider>
          <RouterProvider router={router} />
        </ThemeProvider>
      </QueryClientProvider>
      <ToastManager />
      <ApplicationUpdateManager />
    </TypesafeI18n>
  );
};
