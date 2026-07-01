import { useEffect, useRef } from 'react';
import { EMPTY, Subject, timer } from 'rxjs';
import { switchMap } from 'rxjs/operators';

export function useDeferredCallback(callback: () => void) {
  const delay$ = useRef(new Subject<number>());
  const callbackRef = useRef(callback);

  useEffect(() => {
    const sub = delay$.current
      .pipe(switchMap((ms) => (ms > 0 ? timer(ms) : EMPTY)))
      .subscribe(() => {
        callbackRef.current();
      });
    return () => {
      sub.unsubscribe();
    };
  }, []);

  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  return {
    start: (delayMs: number) => {
      delay$.current.next(delayMs);
    },
    cancel: () => {
      delay$.current.next(0); // cancel current timer
    },
  };
}
