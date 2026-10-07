import { emitTo } from '@tauri-apps/api/event';
import z from 'zod';
import { ConnectModalView } from '../../../../pages/full/OverviewPage/components/ConnectModal/hooks/types';
import { useConnectModal } from '../../../../pages/full/OverviewPage/components/ConnectModal/hooks/useConnectModal';
import { WindowId } from '../../../consts';
import { useConfirmModal } from '../../../hooks/confirmModal/useConfirmModal';
import { api } from '../../../rust-api/api';
import { ConnectionType, type LocationInfo, TauriEvent } from '../../../rust-api/types';
import { ButtonVariant } from '../../Button/types';

export const DEFAULT_CONNECTION_ERROR =
  'One or more external services are unavailable or unreachable. This may be caused by a network issue or a temporary service outage. Please try again later.';

const conflictingConnectionSchema = z.object({
  id: z.number(),
  connection_type: z.enum(ConnectionType),
  name: z.string(),
});

const connectErrorSchema = z.object({
  kind: z.enum(['postureCheckFailed', 'serviceUnavailable', 'routeConflict', 'other']),
  message: z.string(),
  conflicts: z.array(conflictingConnectionSchema).default([]),
});

export type ConnectError = z.infer<typeof connectErrorSchema>;
type ConflictingConnection = z.infer<typeof conflictingConnectionSchema>;

export const parseConnectError = (err: unknown): ConnectError | null => {
  const result = connectErrorSchema.safeParse(err);

  return result.success ? result.data : null;
};

export const handleFullViewConnectError = (
  location: LocationInfo,
  err: unknown,
  connect: () => unknown,
) => {
  const connectError = parseConnectError(err);
  if (location.posture_check_required && connectError?.kind === 'postureCheckFailed') {
    useConnectModal.getState().open({
      location,
      view: ConnectModalView.PostureCheckFail,
      postureError: connectError.message,
    });
  } else if (connectError?.kind === 'routeConflict') {
    openRouteConflictModal(location, connectError.conflicts, connect);
  } else if (connectError?.kind === 'serviceUnavailable') {
    useConnectModal.getState().open({
      location,
      view: ConnectModalView.ConnectionError,
    });
  }
};

export type RouteConflictPayload = {
  location: LocationInfo;
  conflicts: ConflictingConnection[];
};

// The compact window is too small for the modal, so the full view is brought forward to show it.
export const openRouteConflictInFullView = async (payload: RouteConflictPayload) => {
  await api.swapToFullView();
  await emitTo(WindowId.FullView, TauriEvent.RouteConflict, payload);
};

export const openRouteConflictModal = (
  location: LocationInfo,
  conflicts: ConflictingConnection[],
  connect: () => unknown,
) => {
  useConfirmModal.getState().open({
    title: 'Connection unavailable',
    content: (
      <div className="markdown-render">
        <p>
          {location.name} conflicts with another location you're connected to. You can see
          the conflicting locations below.
        </p>
        <ul>
          {conflicts.map((c) => (
            <li key={`${c.connection_type}-${c.id}`}>{c.name}</li>
          ))}
        </ul>
        <p>Disconnect the conflicting location(s) to switch to this one.</p>
      </div>
    ),
    submitProps: {
      variant: ButtonVariant.Primary,
      text: 'Disconnect others & connect',
    },
    onSubmit: async () => {
      await Promise.allSettled(
        conflicts.map((c) =>
          api.disconnect({ connectionType: c.connection_type, locationId: c.id }),
        ),
      );
      await connect();
    },
  });
};
