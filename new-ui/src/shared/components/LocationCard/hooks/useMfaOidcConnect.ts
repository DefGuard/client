import { useQuery } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useState } from 'react';
import { useMfaClientAttempt } from '../../../hooks/useMfaClientAttempt';
import { api } from '../../../rust-api/api';
import {
  classifyOidcPollFailure,
  isMfaPostureError,
  isServiceUnavailable,
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

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const instance = instances?.find((i) => i.id === location.instance_id);

  const { startAttempt } = useMfaClientAttempt();

  const start = useCallback(async () => {
    if (!instance) {
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

      // Registered one at a time: a listener whose `listen()` resolves after the
      // attempt went stale is dropped by `ownListener`, and the check between each
      // stops the chain rather than attaching the rest.
      //
      // The backend brings up the connection itself; completion means connected.
      await attempt.ownListener(
        listen(TauriEvent.MfaOpenIdComplete, () => {
          if (!attempt.tryFinish()) return;
          setIsPolling(false);
          setView(LocationCardViews.Connected);
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
          setPollError(classifyOidcPollFailure(event.payload.error).message);
        }),
      );
    } catch (e) {
      if (!attempt.isLive()) return;
      attempt.abandon();
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

  // Deferring by a tick is what keeps a StrictMode replay to a single attempt:
  // the discarded effect clears its timer before it fires, so only the surviving
  // mount starts. It also lets a fast unmount cancel the start outright rather
  // than merely superseding it.
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
