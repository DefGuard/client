import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { debug } from '@tauri-apps/plugin-log';
import { Fragment, type PropsWithChildren, useEffect } from 'react';
import { WindowId } from '../consts';
import {
  type AddInstanceEventPayload,
  type DeadConnectionDroppedPayload,
  type DeadConnectionReconnectedPayload,
  TauriEvent,
} from '../rust-api/types';

export const TauriEventProvider = ({ children }: PropsWithChildren) => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisteners = Promise.all([
      listen<AddInstanceEventPayload>(TauriEvent.AddInstance, (event) => {
        void debug(`UI Received event AddInstance: ${JSON.stringify(event.payload)}`);
        const windowLabel = getCurrentWindow().label;
        if (windowLabel === WindowId.FullView) {
          const { token, url } = event.payload;
          navigate({
            to: '/full/add/instance',
            search: {
              token,
              url,
            },
          });
        }
      }),
      listen(TauriEvent.ConnectionChanged, (event) => {
        void debug(
          `UI Received event ConnectionChanged: ${JSON.stringify(event.payload)}`,
        );
        void queryClient.invalidateQueries({ queryKey: ['alive-connection'] });
        void queryClient.invalidateQueries({ queryKey: ['active-connection'] });
        void queryClient.invalidateQueries({ queryKey: ['locations'] });
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
        void queryClient.invalidateQueries({ queryKey: ['location-details'] });
        void queryClient.invalidateQueries({ queryKey: ['last-connection'] });
      }),

      listen(TauriEvent.InstanceUpdate, (event) => {
        void debug(`UI Received event InstanceUpdate: ${JSON.stringify(event.payload)}`);
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
        void queryClient.invalidateQueries({ queryKey: ['locations'] });
        void queryClient.invalidateQueries({ queryKey: ['has-any-visible-locations'] });
      }),

      listen(TauriEvent.LocationUpdate, (event) => {
        void debug(`UI Received event LocationUpdate: ${JSON.stringify(event.payload)}`);
        void queryClient.invalidateQueries({ queryKey: ['locations'] });
        void queryClient.invalidateQueries({ queryKey: ['location-details'] });
        void queryClient.invalidateQueries({ queryKey: ['has-any-visible-locations'] });
      }),

      listen(TauriEvent.AppVersionFetch, (event) => {
        void debug(`UI Received event AppVersionFetch: ${JSON.stringify(event.payload)}`);
        void queryClient.invalidateQueries({ queryKey: ['latest-app-version'] });
      }),

      listen(TauriEvent.ConfigChanged, (event) => {
        void debug(`UI Received event ConfigChanged: ${JSON.stringify(event.payload)}`);
        void queryClient.invalidateQueries({ queryKey: ['settings'] });
        void queryClient.invalidateQueries({ queryKey: ['provisioning-config'] });
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
        void queryClient.invalidateQueries({ queryKey: ['has-any-visible-locations'] });
      }),

      listen<DeadConnectionDroppedPayload>(TauriEvent.DeadConnectionDropped, (event) => {
        void debug(
          `UI Received event DeadConnectionDropped: ${JSON.stringify(event.payload)}`,
        );
        void queryClient.invalidateQueries({ queryKey: ['alive-connection'] });
        void queryClient.invalidateQueries({ queryKey: ['active-connection'] });
        void queryClient.invalidateQueries({ queryKey: ['locations'] });
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
      }),

      listen<DeadConnectionReconnectedPayload>(
        TauriEvent.DeadConnectionReconnected,
        (event) => {
          void debug(
            `UI Received event DeadConnectionReconnected: ${JSON.stringify(event.payload)}`,
          );
          void queryClient.invalidateQueries({ queryKey: ['alive-connection'] });
          void queryClient.invalidateQueries({ queryKey: ['active-connection'] });
          void queryClient.invalidateQueries({ queryKey: ['locations'] });
          void queryClient.invalidateQueries({ queryKey: ['instances'] });
        },
      ),

      listen(TauriEvent.ApplicationConfigChanged, (event) => {
        void debug(
          `UI Received event ApplicationConfigChanged: ${JSON.stringify(event.payload)}`,
        );
        void queryClient.invalidateQueries({ queryKey: ['settings'] });
      }),

      listen<AddInstanceEventPayload>(TauriEvent.AddInstance, (event) => {
        void debug(`UI Received event AddInstance: ${JSON.stringify(event.payload)}`);
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
      }),

      listen(TauriEvent.UuidMismatch, (event) => {
        void debug(`UI Received event UuidMismatch: ${JSON.stringify(event.payload)}`);
        void queryClient.invalidateQueries({ queryKey: ['instances'] });
      }),

      listen(TauriEvent.SessionStateChanged, () => {
        void queryClient.invalidateQueries({ queryKey: ['session-state'] });
      }),
    ]);

    return () => {
      void unlisteners.then((fns) => fns.forEach((fn) => void fn()));
    };
  }, [queryClient, navigate]);

  return <Fragment>{children}</Fragment>;
};
