import { useQuery } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { interval } from 'rxjs';
import { api } from '../../../../shared/rust-api/api';
import {
  type DeviceVerificationError,
  type DeviceVerificationErrorPayload,
  type DeviceVerificationResolvedPayload,
  TauriEvent,
} from '../../../../shared/rust-api/types';
import {
  type DeviceVerificationStart,
  toDeviceVerificationError,
} from '../startVerification';

export type DeviceVerificationStatus =
  | { kind: 'pending' }
  /** The administrator cancelled the session. */
  | { kind: 'stopped' }
  | { kind: 'expired' }
  | { kind: 'error'; error: DeviceVerificationError };

type Args = {
  instanceId: number;
  /** Started by the route loader before the page shows. */
  start: DeviceVerificationStart;
  onVerified: () => void;
  onBlocked: () => void;
};

/** Codes are fetched a moment after they expire so the backend is past the boundary. */
const REFETCH_MARGIN_MS = 250;

export const useDeviceVerification = ({
  instanceId,
  start,
  onVerified,
  onBlocked,
}: Args) => {
  const [resolved, setResolved] = useState<DeviceVerificationStatus | null>(null);
  const callbacks = useRef({ onVerified, onBlocked });
  callbacks.current = { onVerified, onBlocked };

  const code = useQuery({
    queryKey: ['device-verification-code', instanceId, start.ok && start.info.session_id],
    queryFn: () => api.getDeviceVerificationCode(instanceId),
    enabled: start.ok && resolved === null,
    initialData: start.ok ? start.info.code : undefined,
    initialDataUpdatedAt: start.ok ? start.fetchedAt : undefined,
    // The code changes on its own schedule, refetchInterval alone drives it.
    staleTime: Infinity,
    gcTime: 0,
    retry: false,
    refetchOnWindowFocus: false,
    refetchInterval: (query) =>
      (query.state.data?.expires_in ?? 1) * 1000 + REFETCH_MARGIN_MS,
  });

  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const sub = interval(1_000).subscribe(() => setNow(Date.now()));
    return () => sub.unsubscribe();
  }, []);
  const secondsLeft = code.data
    ? Math.max(
        0,
        Math.ceil((code.dataUpdatedAt + code.data.expires_in * 1000 - now) / 1000),
      )
    : null;

  useEffect(() => {
    const handleError = (error: DeviceVerificationError) => {
      switch (error.type) {
        case 'device_blocked':
          callbacks.current.onBlocked();
          return;
        case 'cancelled':
          return;
        default:
          setResolved({ kind: 'error', error });
      }
    };

    const unlisteners = Promise.all([
      listen<DeviceVerificationResolvedPayload>(
        TauriEvent.DeviceVerificationResolved,
        ({ payload }) => {
          if (payload.instance_id !== instanceId) return;
          switch (payload.outcome) {
            case 'verified':
              callbacks.current.onVerified();
              break;
            case 'blocked':
              callbacks.current.onBlocked();
              break;
            case 'cancelled':
              setResolved({ kind: 'stopped' });
              break;
            case 'expired':
              setResolved({ kind: 'expired' });
              break;
          }
        },
      ),
      listen<DeviceVerificationErrorPayload>(
        TauriEvent.DeviceVerificationError,
        ({ payload }) => {
          if (payload.instance_id !== instanceId) return;
          handleError(payload.error);
        },
      ),
    ]);
    return () => {
      void unlisteners.then((fns) => {
        for (const unlisten of fns) unlisten();
      });
    };
  }, [instanceId]);

  // Events win: a failed code fetch right before an outcome must not mask it.
  let status: DeviceVerificationStatus = { kind: 'pending' };
  if (resolved) status = resolved;
  else if (!start.ok) status = { kind: 'error', error: start.error };
  else if (code.isError)
    status = { kind: 'error', error: toDeviceVerificationError(code.error) };

  return {
    status,
    code: status.kind === 'pending' ? (code.data?.code ?? null) : null,
    secondsLeft,
  };
};
