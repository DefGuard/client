import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

import { TauriEventKey } from '../../pages/client/types';
import { NotificationStore, useNotificationStore } from './useNotificationStore';

export const NotificationManager = () => {
  const setNotificationData = useNotificationStore((state) => state.setValues);

  // Listen for notifications and push them into store
  useEffect(() => {
    listen(TauriEventKey.CONFIG_CHANGED, (data) => {
      const payload = data.payload as string;
      const state = {
        header: payload,
        text: payload,
        dismissed: false,
      } as NotificationStore;
      setNotificationData(state);
    });
  }, [setNotificationData]);

  return null;
};
