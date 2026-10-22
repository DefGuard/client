import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useResendCooldown } from './useResendCooldown';

describe('useResendCooldown', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts sent and allows a resend after the cooldown', () => {
    const { result } = renderHook(() => useResendCooldown());
    expect(result.current.sent).toBe(true);

    act(() => {
      vi.advanceTimersByTime(4_999);
    });
    expect(result.current.sent).toBe(true);

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.sent).toBe(false);
  });

  it('starts unsent when asked to', () => {
    const { result } = renderHook(() => useResendCooldown(false));
    expect(result.current.sent).toBe(false);
  });

  it('restarts the full cooldown on every send', () => {
    const { result } = renderHook(() => useResendCooldown(false));

    act(() => {
      result.current.restart();
    });
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    act(() => {
      result.current.restart();
    });
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(result.current.sent).toBe(true);

    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    expect(result.current.sent).toBe(false);
  });

  it('reset allows a resend right away', () => {
    const { result } = renderHook(() => useResendCooldown());

    act(() => {
      result.current.reset();
    });
    expect(result.current.sent).toBe(false);

    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(result.current.sent).toBe(false);
  });
});
