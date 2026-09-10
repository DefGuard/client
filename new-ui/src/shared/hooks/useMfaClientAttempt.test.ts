import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMfaClientAttempt } from './useMfaClientAttempt';

const mocks = vi.hoisted(() => ({
  cancelMfa: vi.fn(),
}));

vi.mock('../rust-api/api', () => ({
  api: {
    cancelMfa: mocks.cancelMfa,
  },
}));

describe('useMfaClientAttempt', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.cancelMfa.mockResolvedValue(undefined);
  });

  it('reports a superseded attempt as no longer live', () => {
    const { result } = renderHook(() => useMfaClientAttempt());

    const first = result.current.startAttempt();
    expect(first.isLive()).toBe(true);

    const second = result.current.startAttempt();

    expect(first.isLive()).toBe(false);
    expect(second.isLive()).toBe(true);
  });

  it('lets only the first terminal claim win', () => {
    const { result } = renderHook(() => useMfaClientAttempt());
    const op = result.current.startAttempt();

    expect(op.tryFinish()).toBe(true);
    expect(op.tryFinish()).toBe(false);
    expect(op.isLive()).toBe(false);
  });

  it('drops the tracked listeners when a terminal outcome is claimed', async () => {
    const unlisten = vi.fn();
    const { result } = renderHook(() => useMfaClientAttempt());
    const op = result.current.startAttempt();

    await op.ownListener(Promise.resolve(unlisten));
    op.ownTask('task-1');

    expect(op.tryFinish()).toBe(true);
    expect(unlisten).toHaveBeenCalledTimes(1);
    // A final event means the task is done, so forget it instead of cancelling it.
    expect(mocks.cancelMfa).not.toHaveBeenCalled();
  });

  it('unlistens a listener that resolves after supersession', async () => {
    const staleUnlisten = vi.fn();
    let resolveListener!: (unlisten: () => void) => void;
    const pending = new Promise<() => void>((resolve) => {
      resolveListener = resolve;
    });
    const { result } = renderHook(() => useMfaClientAttempt());
    const first = result.current.startAttempt();
    const tracked = first.ownListener(pending);

    result.current.startAttempt();
    resolveListener(staleUnlisten);
    await tracked;

    expect(staleUnlisten).toHaveBeenCalledTimes(1);
  });

  it('cancels a task handed over after supersession', () => {
    const { result } = renderHook(() => useMfaClientAttempt());
    const first = result.current.startAttempt();

    result.current.startAttempt();
    first.ownTask('late-task');

    expect(mocks.cancelMfa).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('late-task');
  });

  it('drops the listeners and cancels the task of the attempt it supersedes', async () => {
    const unlisten = vi.fn();
    const { result } = renderHook(() => useMfaClientAttempt());
    const first = result.current.startAttempt();

    await first.ownListener(Promise.resolve(unlisten));
    first.ownTask('task-1');
    result.current.startAttempt();

    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('task-1');
  });

  it('abandons listeners and task on request while staying live', async () => {
    const unlisten = vi.fn();
    const { result } = renderHook(() => useMfaClientAttempt());
    const op = result.current.startAttempt();

    await op.ownListener(Promise.resolve(unlisten));
    op.ownTask('task-1');
    op.abandon();

    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('task-1');
    expect(op.isLive()).toBe(true);
  });

  it('ignores an abandon from a superseded attempt', async () => {
    const unlisten = vi.fn();
    const { result } = renderHook(() => useMfaClientAttempt());
    const first = result.current.startAttempt();
    const second = result.current.startAttempt();

    await second.ownListener(Promise.resolve(unlisten));
    second.ownTask('task-2');
    first.abandon();

    expect(unlisten).not.toHaveBeenCalled();
    expect(mocks.cancelMfa).not.toHaveBeenCalled();
  });

  it('cancels an owned task and drops listeners on unmount', async () => {
    const unlisten = vi.fn();
    const { result, unmount } = renderHook(() => useMfaClientAttempt());
    const op = result.current.startAttempt();

    await op.ownListener(Promise.resolve(unlisten));
    op.ownTask('task-1');
    unmount();

    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledTimes(1);
    expect(mocks.cancelMfa).toHaveBeenCalledWith('task-1');
    expect(op.isLive()).toBe(false);
  });

  it('swallows a rejected task cancellation', () => {
    mocks.cancelMfa.mockRejectedValue(new Error('gone'));
    const { result, unmount } = renderHook(() => useMfaClientAttempt());
    const op = result.current.startAttempt();

    op.ownTask('task-1');

    expect(() => unmount()).not.toThrow();
    expect(mocks.cancelMfa).toHaveBeenCalledWith('task-1');
  });
});
