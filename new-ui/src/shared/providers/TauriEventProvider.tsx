import { useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { Fragment, type PropsWithChildren, useEffect } from 'react';

import {
  type AddInstanceEventPayload,
  type DeadConnectionDroppedPayload,
  type DeadConnectionReconnectedPayload,
  TauriEvent,
} from '../rust-api/types';

export const TauriEventProvider = ({ children }: PropsWithChildren) => {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisteners = Promise.all([
      listen(TauriEvent.ConnectionChanged, () => {
        void queryClient.invalidateQueries({ queryKey: ['alive-connection'] });
        void queryClient.invalidateQueries({ queryKey: ['active-connection'] });
        void queryClient.invalidateQueries({ queryKey: ['locations'] });
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
        void queryClient.invalidateQueries({ queryKey: ['location-details'] });
        void queryClient.invalidateQueries({ queryKey: ['last-connection'] });
      }),

      listen(TauriEvent.InstanceUpdate, () => {
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
        void queryClient.invalidateQueries({ queryKey: ['locations'] });
        void queryClient.invalidateQueries({ queryKey: ['has-any-visible-locations'] });
      }),

      listen(TauriEvent.LocationUpdate, () => {
        void queryClient.invalidateQueries({ queryKey: ['locations'] });
        void queryClient.invalidateQueries({ queryKey: ['location-details'] });
        void queryClient.invalidateQueries({ queryKey: ['has-any-visible-locations'] });
      }),

      listen(TauriEvent.AppVersionFetch, () => {
        void queryClient.invalidateQueries({ queryKey: ['latest-app-version'] });
      }),

      listen(TauriEvent.ConfigChanged, () => {
        void queryClient.invalidateQueries({ queryKey: ['settings'] });
        void queryClient.invalidateQueries({ queryKey: ['provisioning-config'] });
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
        void queryClient.invalidateQueries({ queryKey: ['has-any-visible-locations'] });
      }),

      listen<DeadConnectionDroppedPayload>(TauriEvent.DeadConnectionDropped, () => {
        void queryClient.invalidateQueries({ queryKey: ['alive-connection'] });
        void queryClient.invalidateQueries({ queryKey: ['active-connection'] });
        void queryClient.invalidateQueries({ queryKey: ['locations'] });
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
      }),

      listen<DeadConnectionReconnectedPayload>(
        TauriEvent.DeadConnectionReconnected,
        () => {
          void queryClient.invalidateQueries({ queryKey: ['alive-connection'] });
          void queryClient.invalidateQueries({ queryKey: ['active-connection'] });
          void queryClient.invalidateQueries({ queryKey: ['locations'] });
          void queryClient.invalidateQueries({ queryKey: ['instances'] });
        },
      ),

      listen(TauriEvent.ApplicationConfigChanged, () => {
        void queryClient.invalidateQueries({ queryKey: ['settings'] });
      }),

      listen<AddInstanceEventPayload>(TauriEvent.AddInstance, () => {
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
      }),

      listen(TauriEvent.UuidMismatch, () => {
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
      }),
    ]);

    return () => {
      void unlisteners.then((fns) => fns.forEach((fn) => void fn()));
    };
  }, [queryClient]);

  return <Fragment>{children}</Fragment>;
};
