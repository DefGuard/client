import { getVersion } from '@tauri-apps/api/app';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

import { clientApi } from '../../pages/client/clientAPI/clientApi.ts';
import { useClientStore } from '../../pages/client/hooks/useClientStore';
import { TauriEventKey } from '../../pages/client/types';
import { NewApplicationVersionInfo } from '../../shared/hooks/api/types';
import {
  ApplicationUpdateStore,
  useApplicationUpdateStore,
} from './useApplicationUpdateStore';

const { getLatestAppVersion } = clientApi;

export const ApplicationUpdateManager = () => {
  const [appVersion, setAppVersion] = useState<string | undefined>(undefined);

  const setApplicationUpdateData = useApplicationUpdateStore((state) => state.setValues);
  const checkForUpdates = useClientStore((state) => state.settings.check_for_updates);

  useEffect(() => {
    const getAppVersion = async () => {
      const version = await getVersion().catch(() => {
        return '';
      });
      setAppVersion(version);
    };

    getAppVersion();
  }, []);

  useEffect(() => {
    const subs: UnlistenFn[] = [];

    if (!checkForUpdates) {
      subs.forEach((sub) => sub());
      return;
    }

    listen(TauriEventKey.APP_VERSION_FETCH, (data) => {
      const payload = data.payload as NewApplicationVersionInfo;
      const state = {
        latestVersion: payload.version,
        releaseDate: payload.release_date,
        releaseNotesUrl: payload.release_notes_url,
        updateUrl: payload.update_url,
        dismissed: false,
      } as ApplicationUpdateStore;
      setApplicationUpdateData(state);
    }).then((cleanup) => {
      subs.push(cleanup);
    });

    return () => {
      subs.forEach((sub) => sub());
    };
  }, [checkForUpdates, setApplicationUpdateData]);

  useEffect(() => {
    if (!checkForUpdates || !appVersion) return;

    const getNewVersion = async (appVersion: string) => {
      if (!appVersion) return;

      const response = await getLatestAppVersion();

      setApplicationUpdateData({
        currentVersion: appVersion,
        latestVersion: response.version,
        releaseDate: response.release_date,
        releaseNotesUrl: response.release_notes_url,
        updateUrl: response.update_url,
        dismissed: false,
      });
    };

    getNewVersion(appVersion);
  }, [checkForUpdates, appVersion, setApplicationUpdateData]);

  return null;
};
