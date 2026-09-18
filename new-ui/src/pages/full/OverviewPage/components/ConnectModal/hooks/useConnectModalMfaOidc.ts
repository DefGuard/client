import { useQuery } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useState } from 'react';
import { useShallow } from 'zustand/shallow';
import { useMfaClientAttempt } from '../../../../../../shared/hooks/useMfaClientAttempt';
import { api } from '../../../../../../shared/rust-api/api';
import {
  classifyOidcPollFailure,
  isMfaPostureError,
  isServiceUnavailable,
  mfaErrorMessage,
} from '../../../../../../shared/rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../../../../shared/rust-api/query';
import type {
  MfaErrorPayload,
  MfaStepAdvancedPayload,
} from '../../../../../../shared/rust-api/types';
import { MfaMethod, TauriEvent } from '../../../../../../shared/rust-api/types';
import { buildOpenIdMfaUrl } from '../../../../../../shared/utils/openIdMfaUrl';
import { useConnectModal } from './useConnectModal';

type Options = {
  autoStart?: boolean;
  onPostureError?: (msg: string) => void;
  onSessionExpired?: () => void;
  onServiceUnavailable?: () => void;
};

export const useConnectModalMfaOidc = ({
  autoStart = false,
  onPostureError,
  onSessionExpired,
  onServiceUnavailable,
}: Options = {}) => {
  const [location, stepPlan, mfaToken, setMfaToken, goToStep] = useConnectModal(
    useShallow((s) => [s.location, s.stepPlan, s.mfaToken, s.setMfaToken, s.goToStep]),
  );

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [pollError, setPollError] = useState<string | null>(null);

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location?.instance_id);

  const { startAttempt } = useMfaClientAttempt();

  const start = useCallback(async () => {
    if (!instance || !location) {
      setStartError('Instance not found');
      return;
    }

    const attempt = startAttempt();
    setIsStarting(true);
    setStartError(null);
    setPollError(null);

    try {
      const session = await api.mfaBeginStep(
        instance.id,
        location.id,
        MfaMethod.Oidc,
        stepPlan,
        mfaToken,
      );
      if (!attempt.isLive()) return;

      const openIdUrl = buildOpenIdMfaUrl(
        instance.proxy_url,
        session.token,
        session.stepAttemptId,
      );
      await api.openLink(openIdUrl.toString());
      if (!attempt.isLive()) return;
      setMfaToken(session.token);

      setIsStarting(false);
      setIsPolling(true);

      const taskId = await api.mfaPollOpenId(
        instance.id,
        location.id,
        session.token,
        session.stepAttemptId,
      );
      attempt.ownTask(taskId);
      if (!attempt.isLive()) return;

      // Add listeners one at a time so a late listener cannot leave later listeners active.
      //
      // The backend connects the VPN; completion means it is connected.
      await attempt.ownListener(
        listen(TauriEvent.MfaOpenIdComplete, () => {
          if (!attempt.tryFinish()) return;
          setIsPolling(false);
        }),
      );
      if (!attempt.isLive()) return;

      await attempt.ownListener(
        listen<MfaStepAdvancedPayload>(TauriEvent.MfaOpenIdStepAdvanced, (event) => {
          if (!attempt.tryFinish()) return;
          setIsPolling(false);
          goToStep(event.payload.nextStep);
        }),
      );
      if (!attempt.isLive()) return;

      await attempt.ownListener(
        listen<MfaErrorPayload>(TauriEvent.MfaOpenIdError, (event) => {
          if (!attempt.tryFinish()) return;
          setIsPolling(false);
          void error(
            `OIDC MFA failed for location ${location.id}: ${event.payload.error}`,
          );
          const failure = classifyOidcPollFailure(event.payload.error);
          // The full view handles an expired session itself; the compact view shows a message.
          if (failure.kind === 'sessionExpired') {
            onSessionExpired?.();
          } else {
            setPollError(failure.message);
          }
        }),
      );
    } catch (e) {
      if (!attempt.isLive()) return;
      attempt.abandon();
      void error(`OIDC MFA start failed for location ${location.id}: ${e}`);
      if (isMfaPostureError(e, location)) {
        onPostureError?.(mfaErrorMessage(e));
        return;
      }
      if (isServiceUnavailable(e)) {
        onServiceUnavailable?.();
        return;
      }
      setStartError(mfaErrorMessage(e));
    } finally {
      if (attempt.isLive()) {
        setIsStarting(false);
      }
    }
  }, [
    startAttempt,
    instance,
    location,
    stepPlan,
    mfaToken,
    setMfaToken,
    goToStep,
    onPostureError,
    onSessionExpired,
    onServiceUnavailable,
  ]);

  // Wait one tick so React Strict Mode or an immediate unmount can cancel the
  // start before it runs.
  // biome-ignore lint/correctness/useExhaustiveDependencies: auto-start only on mount
  useEffect(() => {
    if (!autoStart) return;

    const timer = window.setTimeout(() => {
      void start();
    }, 0);

    return () => window.clearTimeout(timer);
  }, [autoStart]);

  return { start, isStarting, startError, isPolling, pollError };
};
