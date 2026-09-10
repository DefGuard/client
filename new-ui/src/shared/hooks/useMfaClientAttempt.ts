import type { UnlistenFn } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef } from 'react';
import { api } from '../rust-api/api';

/**
 * One client-side attempt at driving a task-based MFA step, owning the Rust task and
 * the Tauri listeners so a late resolution cannot act after a retry or an unmount.
 *
 * Not the server's `step_attempt_id`: this never leaves the browser, so abandoning it
 * abandons nothing server-side.
 */
export type ClientAttempt = {
  /** True while this is still the newest attempt. */
  isLive: () => boolean;
  /**
   * Claim the attempt's single outcome, retiring it. First caller wins; the rest get
   * `false` and must bail, as does a stale attempt.
   */
  tryFinish: () => boolean;
  /**
   * Attach a listener, unlistening it at once if the attempt went stale while
   * `listen()` was in flight, so a half-registered listener cannot leak.
   */
  ownListener: (listener: Promise<UnlistenFn>) => Promise<UnlistenFn>;
  /** Hand over the Rust task to cancel on cleanup, or at once if already stale. */
  ownTask: (taskId: string) => void;
  /**
   * Drop the listeners and cancel the task **without** retiring, for a caller
   * unwinding its own state while staying live. No-op once stale.
   */
  abandon: () => void;
};

/**
 * Shared async lifecycle for the task-based MFA hooks. `startAttempt()` retires the
 * previous attempt, so a retry, a step advance and an unmount all resolve the same
 * way: whatever the old attempt awaited finds itself stale and stops.
 */
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
    // Retire first, release second: a terminal event arriving from the attempt
    // being torn down must already read as stale.
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
        return unlisten;
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
