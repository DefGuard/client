import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { type PropsWithChildren, useEffect } from 'react';
import { type AddInstancePayload, TauriEventKey } from '../../../pages/client/types';
import useAddInstance from '../../hooks/useAddInstance';
import { errorDetail } from '../../utils/errorDetail';

export const linkStorageKey = 'lastSuccessfullyHandledDeepLink';

export const storeLink = (value: string) => {
  sessionStorage.setItem(linkStorageKey, value);
};

export const DeepLinkProvider = ({ children }: PropsWithChildren) => {
  const { handleAddInstance } = useAddInstance();

  useEffect(() => {
    const unlisten = listen<AddInstancePayload>(TauriEventKey.ADD_INSTANCE, (event) => {
      handleAddInstance(event.payload).catch((e) => {
        error(`Failed to handle add-instance event: ${errorDetail(e)}`);
      });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [handleAddInstance]);

  return <>{children}</>;
};
