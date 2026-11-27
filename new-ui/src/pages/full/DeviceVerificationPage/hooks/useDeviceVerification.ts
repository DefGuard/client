import { useQuery, useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useMemo, useRef, useState } from 'react';
import { interval } from 'rxjs';
import { api } from '../../../../shared/rust-api/api';
import { getInstancesQueryOptions } from '../../../../shared/rust-api/query';
import {
  DeviceStatus,
  type DeviceStatusValue,
  type DeviceVerificationError,
  type DeviceVerificationErrorPayload,
  type DeviceVerificationResolvedPayload,
  TauriEvent,
} from '../../../../shared/rust-api/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import type { DeviceVerificationStart } from '../types';
import { toDeviceVerificationError } from '../utils/toDeviceVerificationError';

export type DeviceVerificationStatus =
  | { kind: 'pending' }
  | { kind: 'cancelled' }
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
const refetchMarginMs = 250;

export const useDeviceVerification = ({
  instanceId,
  start,
  onVerified,
  onBlocked,
}: Args) => {
  const queryClient = useQueryClient();
  const [resolved, setResolved] = useState<DeviceVerificationStatus | null>(null);
  const callbacksRef = useRef({ onVerified, onBlocked });
  useEffect(() => {
    callbacksRef.current = { onVerified, onBlocked };
  }, [onVerified, onBlocked]);

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
      (query.state.data?.expires_in ?? 1) * 1000 + refetchMarginMs,
  });

  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const sub = interval(1_000).subscribe(() => setNow(Date.now()));
    return () => {
      sub.unsubscribe();
    };
  }, []);

  const secondsLeft = useMemo(() => {
    if (!isPresent(code.data)) return null;
    const expiresAt = code.dataUpdatedAt + code.data.expires_in * 1000;
    return Math.max(0, Math.ceil((expiresAt - now) / 1000));
  }, [code.data, code.dataUpdatedAt, now]);

  useEffect(() => {
    // The next instances refetch reconciles this with the backend.
    const setDeviceStatus = (status: DeviceStatusValue) => {
      queryClient.setQueryData(getInstancesQueryOptions.queryKey, (instances) =>
        instances?.map((instance) =>
          instance.id === instanceId ? { ...instance, device_status: status } : instance,
        ),
      );
    };

    const handleError = (error: DeviceVerificationError) => {
      switch (error.type) {
        case 'device_blocked':
          setDeviceStatus(DeviceStatus.Blocked);
          callbacksRef.current.onBlocked();
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
              setDeviceStatus(DeviceStatus.Verified);
              callbacksRef.current.onVerified();
              break;
            case 'blocked':
              setDeviceStatus(DeviceStatus.Blocked);
              callbacksRef.current.onBlocked();
              break;
            case 'cancelled':
              setResolved({ kind: 'cancelled' });
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
  }, [instanceId, queryClient]);

  // Events win: a failed code fetch right before an outcome must not mask it.
  const status = useMemo((): DeviceVerificationStatus => {
    if (isPresent(resolved)) return resolved;
    if (!start.ok) return { kind: 'error', error: start.error };
    if (code.isError)
      return { kind: 'error', error: toDeviceVerificationError(code.error) };
    return { kind: 'pending' };
  }, [resolved, start, code.isError, code.error]);

  return {
    status,
    code: status.kind === 'pending' ? (code.data?.code ?? null) : null,
    secondsLeft,
  };
};
