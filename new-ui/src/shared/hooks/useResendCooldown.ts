import { useCallback, useEffect, useRef, useState } from 'react';

const resendCooldownMs = 5_000;

/** The first email goes out automatically, so by default the cooldown runs from mount. */
export const useResendCooldown = (initiallySent = true) => {
  const [sent, setSent] = useState(initiallySent);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clear = useCallback(() => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  const restart = useCallback(() => {
    clear();
    setSent(true);
    timeoutRef.current = setTimeout(() => {
      timeoutRef.current = null;
      setSent(false);
    }, resendCooldownMs);
  }, [clear]);

  const reset = useCallback(() => {
    clear();
    setSent(false);
  }, [clear]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: runs on mount only
  useEffect(() => {
    if (initiallySent) restart();
    return clear;
  }, []);

  return { sent, restart, reset };
};
