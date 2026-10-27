import { useEffect, useState } from 'react';

export const useTextOverflow = <T extends HTMLElement = HTMLElement>() => {
  const [element, setElement] = useState<T | null>(null);
  const [isOverflowing, setOverflowing] = useState(false);

  useEffect(() => {
    if (!element) return;
    const observer = new ResizeObserver(() => {
      setOverflowing(element.scrollWidth > element.clientWidth);
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [element]);

  return { ref: setElement, isOverflowing };
};
