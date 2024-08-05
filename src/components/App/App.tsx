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
import { useEffect, useMemo, useState } from 'react';
import { createBrowserRouter, Navigate, RouterProvider } from 'react-router-dom';
import { debug } from 'tauri-plugin-log-api';
import { localStorageDetector } from 'typesafe-i18n/detectors';

import TypesafeI18n from '../../i18n/i18n-react';
import { detectLocale } from '../../i18n/i18n-util';
import { loadLocaleAsync } from '../../i18n/i18n-util.async';
import { ClientPage } from '../../pages/client/ClientPage';
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
import { SplashPage } from '../../pages/splash/SplashPage';
import { ToastManager } from '../../shared/defguard-ui/components/Layout/ToastManager/ToastManager';
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

const router = createBrowserRouter([
  {
    index: true,
    element: <SplashPage />,
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
  const [localeLoaded, setWasLoaded] = useState(false);

  const appLoaded = useMemo(() => localeLoaded, [localeLoaded]);

  // load locales
  useEffect(() => {
    debug('Loading locales');
    loadLocaleAsync(detectedLocale).then(() => {
      setWasLoaded(true);
      debug(`Locale ${detectedLocale} loaded.`);
    });
    dayjs.locale(detectedLocale);
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
