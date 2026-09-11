import type { UnlistenFn } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef } from 'react';
import { api } from '../rust-api/api';

/**
 * Tracks one MFA request and cleans up its background task and listeners when it
 * finishes, is replaced, or unmounts. This is separate from the server's
 * `step_attempt_id`.
 */
type ClientAttempt = {
  /** True while this is still the newest attempt. */
  isLive: () => boolean;
  /** Takes the first final result for this attempt. Later calls fail. */
  tryFinish: () => boolean;
  /** Adds a listener, or removes it immediately if the attempt is no longer current. */
  ownListener: (listener: Promise<UnlistenFn>) => Promise<void>;
  /** Tracks the task for cleanup, or cancels it immediately if the attempt is old. */
  ownTask: (taskId: string) => void;
  /** Stops this attempt's listeners and task without marking it finished. Does nothing if it is no longer current. */
  abandon: () => void;
};

/** Shared lifecycle for task-based MFA hooks. Starting a new attempt stops the previous one. */
export const useMfaClientAttempt = () => {
  const liveAttemptRef = useRef(0);
  const taskIdRef = useRef<string | null>(null);
  const listenersRef = useRef<UnlistenFn[]>([]);

  const cancelTask = useCallback((taskId: string) => {
    void api.cancelMfa(taskId).catch(() => {});
  }, []);

  const dropListeners = useCallback(() => {
    for (const unlisten of listenersRef.current.splice(0)) {
      unlisten();
    }
  }, []);

  const releaseHeld = useCallback(() => {
    dropListeners();
    const taskId = taskIdRef.current;
    taskIdRef.current = null;
    if (taskId !== null) {
      cancelTask(taskId);
    }
  }, [cancelTask, dropListeners]);

  const startAttempt = useCallback((): ClientAttempt => {
    // Mark the old request inactive before cleanup, so a final event cannot use it.
    const serial = ++liveAttemptRef.current;
    releaseHeld();

    const isLive = () => liveAttemptRef.current === serial;

    return {
      isLive,
      tryFinish: () => {
        if (!isLive()) return false;
        liveAttemptRef.current += 1;
        taskIdRef.current = null;
        dropListeners();
        return true;
      },
      ownListener: async (listener) => {
        const unlisten = await listener;
        if (isLive()) {
          listenersRef.current.push(unlisten);
        } else {
          unlisten();
        }
      },
      ownTask: (taskId) => {
        if (isLive()) {
          taskIdRef.current = taskId;
        } else {
          cancelTask(taskId);
        }
      },
      abandon: () => {
        if (!isLive()) return;
        releaseHeld();
      },
    };
  }, [cancelTask, dropListeners, releaseHeld]);

  useEffect(() => {
    return () => {
      liveAttemptRef.current += 1;
      releaseHeld();
    };
  }, [releaseHeld]);

  return { startAttempt };
};
