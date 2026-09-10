import type { UnlistenFn } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef } from 'react';
import { api } from '../rust-api/api';

/**
 * One client-side attempt at driving a task-based MFA step: the Rust task it owns
 * and the Tauri listeners it attached. Every hook that drives such a method has to
 * survive a late resolution landing after the user retried or navigated away, and
 * this is the shape of that guard.
 *
 * Distinct from the server's `step_attempt_id`, which is minted by Core, travels on
 * the wire and binds a proof. This one never leaves the browser and Core knows
 * nothing about it: abandoning it locally does not abandon anything server-side.
 */
export type ClientAttempt = {
  /** True while this is still the newest attempt, so its work is worth doing. */
  isLive: () => boolean;
  /**
   * Take the attempt's single outcome: retires it, drops the listeners and forgets
   * the task. The first of several terminal listeners wins; every later one gets
   * `false` and must bail. Returns `false` for an attempt that is already stale.
   */
  tryFinish: () => boolean;
  /**
   * Attach a listener to this attempt. If the attempt went stale while `listen()`
   * was in flight the listener is unlistened at once rather than recorded, so a
   * half-registered listener can never leak.
   */
  ownListener: (listener: Promise<UnlistenFn>) => Promise<UnlistenFn>;
  /**
   * Hand the attempt the Rust task it should cancel on cleanup. A task that arrives
   * after the attempt went stale is cancelled immediately instead.
   */
  ownTask: (taskId: string) => void;
  /**
   * Drop this attempt's listeners and cancel its task **without** retiring it, for
   * a caller that failed outright but still has state of its own to unwind while
   * remaining the live attempt. A no-op once the attempt is stale.
   */
  abandon: () => void;
};

/**
 * Shared async lifecycle for the task-based MFA hooks. `startAttempt()` opens a new
 * attempt and retires the previous one, so a retry, a step advance and an unmount
 * all resolve the same way: whatever the old attempt was still waiting on finds
 * itself stale and stops.
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
