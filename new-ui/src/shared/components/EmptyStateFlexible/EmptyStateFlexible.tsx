import './style.scss';
import { useWindowSize } from '@uidotdev/usehooks';
import { useMemo, useRef } from 'react';
import { EmptyState } from '../EmptyState/EmptyState';
import type { EmptyStateProps } from '../EmptyState/types';

type Props = EmptyStateProps;

export const EmptyStateFlexible = (props: Props) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const windowHeight = useWindowSize().height;

  const initHeight = useMemo(() => {
    if (!containerRef.current) return 0;
    return window.innerHeight - containerRef.current.getBoundingClientRect().top;
  }, []);

  const minHeight = useMemo(() => {
    const container = containerRef.current;
    if (!container || !windowHeight) return null;
    return windowHeight - container.getBoundingClientRect().top;
  }, [windowHeight]);

  return (
    <div
      className="flexible-empty-state"
      ref={containerRef}
      style={{
        minHeight: minHeight ?? initHeight,
      }}
    >
      <EmptyState {...props} />
    </div>
  );
};
