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
import { useEffect, useState } from 'react';
import { createBrowserRouter, Navigate, RouterProvider } from 'react-router-dom';
import { debug } from 'tauri-plugin-log-api';
import { localStorageDetector } from 'typesafe-i18n/detectors';

import TypesafeI18n from '../../i18n/i18n-react';
import { detectLocale } from '../../i18n/i18n-util';
import { loadLocaleAsync } from '../../i18n/i18n-util.async';
import { ClientPage } from '../../pages/client/ClientPage';
import { ClientAddInstancePage } from '../../pages/client/pages/ClientAddInstancePage/ClientAddInstnacePage';
import { ClientInstancePage } from '../../pages/client/pages/ClientInstancePage/ClientInstancePage';
import { EnrollmentPage } from '../../pages/enrollment/EnrollmentPage';
import { SessionTimeoutPage } from '../../pages/sessionTimeout/SessionTimeoutPage';
import { ToastManager } from '../../shared/defguard-ui/components/Layout/ToastManager/ToastManager';
import { routes } from '../../shared/routes';

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
        element: <ClientInstancePage />,
      },
      {
        path: '/client/add-instance',
        element: <ClientAddInstancePage />,
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
  const [wasLoaded, setWasLoaded] = useState(false);

  useEffect(() => {
    debug('Loading locales');
    loadLocaleAsync(detectedLocale).then(() => {
      setWasLoaded(true);
      debug(`Locale ${detectedLocale} loaded.`);
    });
    dayjs.locale(detectedLocale);
  }, []);

  if (!wasLoaded) return null;

  return (
    <TypesafeI18n locale={detectedLocale}>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
      <ToastManager />
    </TypesafeI18n>
  );
};
