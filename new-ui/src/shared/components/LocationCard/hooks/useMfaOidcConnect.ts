import { useQuery } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useMfaClientAttempt } from '../../../hooks/useMfaClientAttempt';
import { api } from '../../../rust-api/api';
import {
  classifyOidcPollFailure,
  isMfaPostureError,
  isServiceUnavailable,
  isTimeout,
  mfaErrorMessage,
} from '../../../rust-api/mfaError';
import { getInstancesQueryOptions } from '../../../rust-api/query';
import type { MfaErrorPayload, MfaStepAdvancedPayload } from '../../../rust-api/types';
import { MfaMethod, TauriEvent } from '../../../rust-api/types';
import { buildOpenIdMfaUrl } from '../../../utils/openIdMfaUrl';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';

export const useMfaOidcConnect = (autoStart = false) => {
  const {
    location,
    setPostureError,
    setView,
    stepPlan,
    mfaToken,
    setMfaToken,
    goToStep,
  } = useLocationCardContext();

  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [pollError, setPollError] = useState<string | null>(null);
  const isPollingRef = useRef(false);

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location.instance_id);

  const { startAttempt } = useMfaClientAttempt();

  useEffect(
    () => () => {
      if (isPollingRef.current) setMfaToken(null);
    },
    [setMfaToken],
  );

  const start = useCallback(async () => {
    if (!instance) {
      setStartError('Instance not found');
      return;
    }

    const replacedPollingAttempt = isPollingRef.current;
    const attempt = startAttempt();
    if (replacedPollingAttempt) {
      isPollingRef.current = false;
      setIsPolling(false);
      setMfaToken(null);
    }
    setIsStarting(true);
    setStartError(null);
    setPollError(null);
    let beginningStep = true;

    try {
      const session = await api.mfaBeginStep(
        instance.id,
        location.id,
        MfaMethod.Oidc,
        stepPlan,
        replacedPollingAttempt ? null : mfaToken,
      );
      beginningStep = false;
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
      isPollingRef.current = true;
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
          isPollingRef.current = false;
          setIsPolling(false);
          setView(LocationCardViews.Connected);
        }),
      );
      if (!attempt.isLive()) return;

      await attempt.ownListener(
        listen<MfaStepAdvancedPayload>(TauriEvent.MfaOpenIdStepAdvanced, (event) => {
          if (!attempt.tryFinish()) return;
          isPollingRef.current = false;
          setIsPolling(false);
          goToStep(event.payload.nextStep);
        }),
      );
      if (!attempt.isLive()) return;

      await attempt.ownListener(
        listen<MfaErrorPayload>(TauriEvent.MfaOpenIdError, (event) => {
          if (!attempt.tryFinish()) return;
          isPollingRef.current = false;
          setIsPolling(false);
          void error(
            `OIDC MFA failed for location ${location.id}: ${event.payload.error}`,
          );
          const failure = classifyOidcPollFailure(event.payload.error);
          if (failure.kind !== 'timeout' && !isServiceUnavailable(event.payload.error)) {
            setMfaToken(null);
          }
          setPollError(failure.message);
        }),
      );
    } catch (e) {
      if (!attempt.isLive()) return;
      attempt.abandon();
      const retryable = isTimeout(e) || isServiceUnavailable(e);
      if (beginningStep && !retryable) setMfaToken(null);
      if (isPollingRef.current) {
        isPollingRef.current = false;
        setIsPolling(false);
        if (!retryable) setMfaToken(null);
      }
      void error(`OIDC MFA start failed for location ${location.id}: ${e}`);
      if (isMfaPostureError(e, location)) {
        setPostureError(mfaErrorMessage(e));
        setView(LocationCardViews.PostureCheckFail);
        return;
      }
      if (isServiceUnavailable(e)) {
        setView(LocationCardViews.ConnectionError);
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
    setPostureError,
    setView,
    goToStep,
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
