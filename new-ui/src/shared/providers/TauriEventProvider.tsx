import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { debug } from '@tauri-apps/plugin-log';
import { Fragment, type PropsWithChildren, useEffect } from 'react';
import { mfaMethodToConnectModalView } from '../../pages/full/OverviewPage/components/ConnectModal/hooks/types';
import { useConnectModal } from '../../pages/full/OverviewPage/components/ConnectModal/hooks/useConnectModal';
import { WindowId } from '../consts';
import { useAppData } from '../providers/AppDataContext';
import { api } from '../rust-api/api';
import {
  type AddInstanceEventPayload,
  ConnectionType,
  type DeadConnectionDroppedPayload,
  type DeadConnectionReconnectedPayload,
  type LocationInfo,
  MfaMethod,
  TauriEvent,
} from '../rust-api/types';
import { useAppStore } from '../store/useAppStore';
import { decideLocationMfaMethod } from '../utils/decideLocationMfaMethod';

export const TauriEventProvider = ({ children }: PropsWithChildren) => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { setViewSelection } = useAppData();

  useEffect(() => {
    const unlisteners = Promise.all([
      listen<AddInstanceEventPayload>(TauriEvent.AddInstance, (event) => {
        void debug(`UI Received event AddInstance (${event.payload.url})`);
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
      // Backend requests the MFA flow (e.g. connecting to an MFA location from the
      // tray or system settings). The location is emitted as the payload directly.
      // The backend targets whichever window it surfaced: the full view (if it was
      // already open) or the compact tray window. Each window handles the event in
      // the way native to it.
      listen<Omit<LocationInfo, 'connection_type' | 'active'>>(
        TauriEvent.MfaTrigger,
        (event) => {
          void debug(`UI Received event MfaTrigger: ${JSON.stringify(event.payload)}`);
          const windowLabel = getCurrentWindow().label;

          // Compact window: select the location's instance, expand its card and
          // flag it to auto-start the MFA flow so the user can enter their code inline.
          if (windowLabel === WindowId.CompactView) {
            const { id: locationId, instance_id: instanceId } = event.payload;
            setViewSelection({ kind: 'instance', id: instanceId });
            useAppStore.setState({
              expandedLocation: locationId,
              mfaAutoStartLocationId: locationId,
            });
            return;
          }

          // Full view: open the shared ConnectModal on the overview page. The
          // location arrives without `connection_type`/`active` (MFA is location-only).
          if (windowLabel === WindowId.FullView) {
            const location: LocationInfo = {
              ...event.payload,
              connection_type: ConnectionType.Location,
              active: false,
            };
            void (async () => {
              const appConfig = await api.getAppConfig();
              const mfaMethod =
                decideLocationMfaMethod(location, location.mfa_method) ?? MfaMethod.Totp;

              // The ConnectModal is only mounted on the overview page.
              await navigate({ to: '/full/overview' });
              useConnectModal.getState().open({
                view: mfaMethodToConnectModalView(mfaMethod),
                location,
                autoStartOpenId: appConfig.auto_start_openid_mfa,
                mfaMethod,
              });
            })();
          }
        },
      ),

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
        void queryClient.invalidateQueries({ queryKey: ['tunnels'] });
        void queryClient.invalidateQueries({ queryKey: ['tunnel-details'] });
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
  }, [queryClient, navigate, setViewSelection]);

  return <Fragment>{children}</Fragment>;
};
