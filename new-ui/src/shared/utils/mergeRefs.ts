// extracted from https://github.com/gregberge/react-merge-refs
import type { Ref, RefCallback } from 'react';

function assignRef<T>(
  ref: Ref<T> | undefined | null,
  value: T | null,
): ReturnType<RefCallback<T>> {
  if (typeof ref === 'function') {
    return ref(value);
  } else if (ref) {
    ref.current = value;
  }
}

export function mergeRefs<T>(refs: (Ref<T> | undefined)[]): Ref<T> {
  return (value: T | null) => {
    const cleanups: (() => void)[] = [];

    for (const ref of refs) {
      const cleanup = assignRef(ref, value);
      const isCleanup = typeof cleanup === 'function';
      cleanups.push(isCleanup ? cleanup : () => assignRef(ref, null));
    }

    return () => {
      for (const cleanup of cleanups) cleanup();
    };
  };
}
