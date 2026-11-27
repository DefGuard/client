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
  type DeviceVerificationOutcome,
  type DeviceVerificationResolvedPayload,
  TauriEvent,
} from '../../../../shared/rust-api/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import type { DeviceVerificationSession } from '../types';
import { toDeviceVerificationError } from '../utils/toDeviceVerificationError';

export type DeviceVerificationStatus =
  | { kind: 'pending' }
  | { kind: 'cancelled' }
  | { kind: 'expired' }
  | { kind: 'error'; error: DeviceVerificationError };

type Args = {
  instanceId: number;
  /** Started by the route loader before the page shows. */
  start: DeviceVerificationSession;
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

  const sessionId = start.info.session_id;
  const codeQueryKey = useMemo(
    () => ['device-verification-code', instanceId, sessionId] as const,
    [instanceId, sessionId],
  );
  const code = useQuery({
    queryKey: codeQueryKey,
    queryFn: () => api.getDeviceVerificationCode(instanceId),
    enabled: resolved === null,
    initialData: start.info.code,
    initialDataUpdatedAt: start.fetchedAt,
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

  const hasLeftRef = useRef(false);

  const settle = useMemo(() => {
    // The next instances refetch reconciles this with the backend.
    const setDeviceStatus = (status: DeviceStatusValue) => {
      queryClient.setQueryData(getInstancesQueryOptions.queryKey, (instances) =>
        instances?.map((instance) =>
          instance.id === instanceId ? { ...instance, device_status: status } : instance,
        ),
      );
    };

    const leave = (status: DeviceStatusValue, callback: () => void) => {
      if (hasLeftRef.current) return;
      hasLeftRef.current = true;
      setDeviceStatus(status);
      callback();
    };

    const applyOutcome = (outcome: DeviceVerificationOutcome) => {
      switch (outcome) {
        case 'verified':
          leave(DeviceStatus.Verified, callbacksRef.current.onVerified);
          break;
        case 'blocked':
          leave(DeviceStatus.Blocked, callbacksRef.current.onBlocked);
          break;
        case 'cancelled':
          setResolved({ kind: 'cancelled' });
          break;
        case 'expired':
          setResolved({ kind: 'expired' });
          break;
      }
    };

    const handleError = (error: DeviceVerificationError) => {
      switch (error.type) {
        case 'device_blocked':
          leave(DeviceStatus.Blocked, callbacksRef.current.onBlocked);
          return;
        case 'cancelled':
          return;
        case 'resolved':
          applyOutcome(error.outcome);
          return;
        default:
          setResolved({ kind: 'error', error });
      }
    };

    return { applyOutcome, handleError };
  }, [instanceId, queryClient]);

  useEffect(() => {
    let isSubscribed = true;
    const unlisteners = Promise.all([
      listen<DeviceVerificationResolvedPayload>(
        TauriEvent.DeviceVerificationResolved,
        ({ payload }) => {
          if (payload.instance_id !== instanceId) return;
          settle.applyOutcome(payload.outcome);
        },
      ),
      listen<DeviceVerificationErrorPayload>(
        TauriEvent.DeviceVerificationError,
        ({ payload }) => {
          if (payload.instance_id !== instanceId) return;
          settle.handleError(payload.error);
        },
      ),
    ]);
    // The backend keeps an outcome sent before the listeners existed, one fetch reports it.
    void unlisteners.then(() => {
      if (!isSubscribed) return;
      void queryClient.refetchQueries({ queryKey: codeQueryKey, exact: true });
    });
    return () => {
      isSubscribed = false;
      void unlisteners.then((fns) => {
        for (const unlisten of fns) unlisten();
      });
    };
  }, [instanceId, codeQueryKey, settle, queryClient]);

  // Events win: a failed code fetch right after an outcome must not mask it.
  useEffect(() => {
    if (!code.isError || isPresent(resolved)) return;
    settle.handleError(toDeviceVerificationError(code.error));
  }, [code.isError, code.error, resolved, settle]);

  const status = useMemo(
    (): DeviceVerificationStatus => resolved ?? { kind: 'pending' },
    [resolved],
  );

  return {
    status,
    code: status.kind === 'pending' ? (code.data?.code ?? null) : null,
    secondsLeft,
  };
};
